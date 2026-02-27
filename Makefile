.PHONY: check test simulate build verify release-check fmt clippy install-local

check:
	cargo check --all-targets

test:
	cargo test

simulate:
	./scripts/simulate_cli_use_cases.sh

build:
	cargo build --release

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets

verify: check test simulate

release-check: check test simulate
	cargo clippy --all-targets -- -D warnings
	cargo build --release

install-local:
	./scripts/install.sh --method path --force
