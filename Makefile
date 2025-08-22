default:
	echo "default command, nothing to do. provide specific command to use run"

claude:
	npx @anthropic-ai/claude-code

ci-cargo-cmds:
	cargo test --verbose
	CI=true cargo test --verbose
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings
