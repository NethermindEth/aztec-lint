.DEFAULT_GOAL := help

.PHONY: help ci quality check check-all-features test test-all-features lint lint-all-features fmt matrix perf generate

help: ## Show available Make targets
	@awk 'BEGIN {FS = ":.*## "; print "Available targets:"} /^[a-zA-Z0-9_.-]+:.*## / {printf "  %-22s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

ci: quality matrix perf generate ## Run full CI gate set

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

matrix: ## Run diagnostics/fix regression and matrix harness suites
	cargo test -p aztec-lint-core regression_ --locked
	cargo test -p aztec-lint-rules scoped_ --locked
	cargo test -p aztec-lint-cli --test cli_golden check_json_output_is_deterministic --locked
	cargo test -p aztec-lint-cli --test cli_golden check_sarif_output_is_deterministic --locked
	cargo test -p aztec-lint-cli --test cli_golden check_text_output_is_deterministic --locked
	cargo test -p aztec-lint-cli --test ui_matrix --locked
	cargo test -p aztec-lint-cli --test fix_matrix --locked
	cargo test -p aztec-lint-cli --test corpus_matrix --locked

perf: ## Run performance budget gate
	cargo xtask perf-gate --check --locked

generate: ## Verify generated catalog/docs artifacts are up to date
	cargo xtask update-lints --check --locked
	cargo xtask docs-portal --check
