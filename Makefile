# Colors for output
RED = \033[0;31m
GREEN = \033[0;32m
YELLOW = \033[1;33m
NC = \033[0m # No Color

# Load .env file if it exists
ifneq ("$(wildcard .env)","")
  include .env
  export
endif

CARGO := cargo
RUSTUP := rustup
BINARY_NAME = api
DOCKER = docker

# === Development Commands ===

.PHONY: audit
audit:
	$(CARGO) audit

.PHONY: lint
lint:
	$(CARGO) clippy --all-targets --all-features -- -D warnings

.PHONY: fmt
fmt:
	$(CARGO) fmt --all --check

.PHONY: fmt/fix
fmt/fix:
	$(CARGO) fmt --all

.PHONY: build
build:
	$(CARGO) build

.PHONY: unit-test
unit-test:
	$(CARGO) test
