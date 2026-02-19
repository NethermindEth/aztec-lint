.DEFAULT_GOAL := help

.PHONY: help ci quality check check-all-features test test-all-features lint lint-all-features fmt

help: ## Show available Make targets
	@awk 'BEGIN {FS = ":.*## "; print "Available targets:"} /^[a-zA-Z0-9_.-]+:.*## / {printf "  %-22s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

ci: quality ## Run full CI quality gate

quality: fmt check check-all-features test test-all-features lint lint-all-features ## Run formatting, checks, tests, and lints across feature sets

check: ## Run cargo check with default feature set
	cargo check --workspace --locked

check-all-features: ## Run cargo check with all features enabled
	cargo check --workspace --all-features --locked

test: ## Run cargo test with default feature set
	cargo test --workspace --locked

test-all-features: ## Run cargo test with all features enabled
	cargo test --workspace --all-features --locked

lint: ## Run clippy with default feature set
	cargo clippy --workspace --all-targets --locked -- -D warnings

lint-all-features: ## Run clippy with all features enabled
	cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

fmt: ## Verify formatting
	cargo fmt --all --check
