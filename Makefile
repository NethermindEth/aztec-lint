.PHONY: ci check test lint

ci: check test lint

check:
	cargo check --workspace --locked

test:
	cargo test --workspace --locked

lint:
	cargo clippy --workspace --all-targets --locked -- -D warnings
