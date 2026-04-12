SHELL := /bin/sh

BIN ?= ds4
CARGO ?= cargo
CARGO_INSTALL_ROOT ?=
RUSTUP_INIT_ARGS ?= -y --profile default
RUSTUP_INSTALLER_URL ?= https://sh.rustup.rs

.PHONY: help init ensure-cargo fetch build install-global check test fmt

help:
	@printf '%s\n' \
		'Targets:' \
		'  make init     Install Rust if needed and build the local repo binary' \
		'  make build    Build the project with the locked dependency set' \
		'  make install-global  Install the ds4 command globally from this checkout' \
		'  make check    Run cargo check' \
		'  make test     Run cargo test' \
		'  make fmt      Format Rust sources'

init: ensure-cargo fetch build
	@printf '%s\n' 'Environment is ready. Try: ./target/debug/$(BIN) run'

ensure-cargo:
	@set -eu; \
	if [ -f "$$HOME/.cargo/env" ]; then . "$$HOME/.cargo/env"; fi; \
	if command -v "$(CARGO)" >/dev/null 2>&1; then \
		"$(CARGO)" --version; \
		exit 0; \
	fi; \
	if ! command -v curl >/dev/null 2>&1; then \
		printf '%s\n' 'cargo is not installed. Install Rust or install curl, then rerun make init.' >&2; \
		exit 1; \
	fi; \
	printf '%s\n' 'cargo not found. Installing Rust with rustup...'; \
	curl --proto '=https' --tlsv1.2 -sSf "$(RUSTUP_INSTALLER_URL)" | sh -s -- $(RUSTUP_INIT_ARGS); \
	. "$$HOME/.cargo/env"; \
	"$(CARGO)" --version

fetch: ensure-cargo
	@set -eu; \
	if [ -f "$$HOME/.cargo/env" ]; then . "$$HOME/.cargo/env"; fi; \
	"$(CARGO)" fetch --locked

build: ensure-cargo
	@set -eu; \
	if [ -f "$$HOME/.cargo/env" ]; then . "$$HOME/.cargo/env"; fi; \
	"$(CARGO)" build --locked

install-global: fetch
	@set -eu; \
	if [ -f "$$HOME/.cargo/env" ]; then . "$$HOME/.cargo/env"; fi; \
	if [ -n "$(CARGO_INSTALL_ROOT)" ]; then \
		"$(CARGO)" install --path . --bin "$(BIN)" --force --locked --offline --root "$(CARGO_INSTALL_ROOT)"; \
	else \
		"$(CARGO)" install --path . --bin "$(BIN)" --force --locked --offline; \
	fi

check: ensure-cargo
	@set -eu; \
	if [ -f "$$HOME/.cargo/env" ]; then . "$$HOME/.cargo/env"; fi; \
	"$(CARGO)" check --locked

test: ensure-cargo
	@set -eu; \
	if [ -f "$$HOME/.cargo/env" ]; then . "$$HOME/.cargo/env"; fi; \
	"$(CARGO)" test --locked

fmt: ensure-cargo
	@set -eu; \
	if [ -f "$$HOME/.cargo/env" ]; then . "$$HOME/.cargo/env"; fi; \
	"$(CARGO)" fmt --all
