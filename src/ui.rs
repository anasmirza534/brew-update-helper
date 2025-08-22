use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::io::{self, Write};

use crate::brew::{OutdatedPackage, PackageType};

pub struct TerminalGuard;

impl TerminalGuard {
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        Ok(TerminalGuard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = io::stdout().flush();
    }
}

pub fn show_interactive_selection(packages: &[&OutdatedPackage]) -> Result<Vec<OutdatedPackage>> {
    // Skip TUI in test environments to avoid terminal state issues
    if std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("CARGO_TEST").is_ok()
        || cfg!(test)
    {
        return show_simple_selection(packages);
    }

    // Track selection state
    let mut selected: Vec<bool> = vec![true; packages.len()];
    let mut list_state = ListState::default();
    list_state.select(Some(0));

    // Setup terminal with proper cleanup handling
    let _guard = TerminalGuard::new()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(2),
                ])
                .split(f.size());

            // Header
            let header = Paragraph::new("Outdated packages found - Select packages to upgrade")
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, chunks[0]);

            // Package list
            let items: Vec<ListItem> = packages
                .iter()
                .enumerate()
                .map(|(i, pkg)| {
                    let checkbox = if selected[i] { "[x]" } else { "[ ]" };
                    let type_str = match pkg.package_type {
                        PackageType::Formula => "Formula",
                        PackageType::Cask => "Cask",
                    };

                    let type_text = format!("({}) ", type_str);
                    let version_text =
                        format!("{} → {}", pkg.current_version, pkg.available_version);

                    let content = Line::from(vec![
                        Span::styled(checkbox, Style::default().fg(Color::Green)),
                        Span::raw(" "),
                        Span::styled(&pkg.name, Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" "),
                        Span::styled(type_text, Style::default().fg(Color::Blue)),
                        Span::raw(version_text),
                    ]);

                    ListItem::new(content)
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL))
                .highlight_style(Style::default().bg(Color::DarkGray));

            f.render_stateful_widget(list, chunks[1], &mut list_state);

            // Footer
            let footer = Paragraph::new("↑↓: Navigate, SPACE: Toggle, ENTER: Proceed, q: Quit")
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => {
                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                        return Ok(vec![]);
                    }
                    KeyCode::Up => {
                        let i = list_state.selected().unwrap_or(0);
                        if i > 0 {
                            list_state.select(Some(i - 1));
                        }
                    }
                    KeyCode::Down => {
                        let i = list_state.selected().unwrap_or(0);
                        if i < packages.len() - 1 {
                            list_state.select(Some(i + 1));
                        }
                    }
                    KeyCode::Char(' ') => {
                        if let Some(i) = list_state.selected() {
                            selected[i] = !selected[i];
                        }
                    }
                    KeyCode::Enter => {
                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                        let result = packages
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| selected[*i])
                            .map(|(_, pkg)| (*pkg).clone())
                            .collect();
                        return Ok(result);
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn show_simple_selection(packages: &[&OutdatedPackage]) -> Result<Vec<OutdatedPackage>> {
    println!("\nOutdated packages found:");

    for (i, pkg) in packages.iter().enumerate() {
        let type_str = match pkg.package_type {
            PackageType::Formula => "Formula",
            PackageType::Cask => "Cask",
        };
        println!(
            "{}. [x] {} ({}) {} → {}",
            i + 1,
            pkg.name,
            type_str,
            pkg.current_version,
            pkg.available_version
        );
    }

    println!("\nAll packages are selected by default.");
    println!(
        "Do you want to proceed with upgrading all {} packages? (y/n): ",
        packages.len()
    );

    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase().starts_with('y') {
        Ok(packages.iter().map(|pkg| (*pkg).clone()).collect())
    } else {
        Ok(vec![])
    }
}
