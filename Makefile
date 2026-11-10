.PHONY: build test clippy fmt fmt-check release install clean check

# Default target
all: fmt-check clippy test

## Build debug binary
build:
	cargo build

## Build optimized release binary
release:
	cargo build --release -p funcspec-cli
	@echo "Binary at: target/release/funcspec"

## Build release AND install it onto PATH.
## The published install.sh COPIES the binary into the install dir (a standalone
## file, NOT a symlink to target/release), so `make release` alone never updates
## an already-installed funcspec. `make install` builds, then copies it into place.
## Install dir (matches install.sh): INSTALL_DIR override wins; else /usr/local/bin
## if writable; else ~/.local/bin.
INSTALL_DIR ?= $(shell if [ -w /usr/local/bin ]; then echo /usr/local/bin; else echo $(HOME)/.local/bin; fi)

install: release
	@mkdir -p "$(INSTALL_DIR)"
	@install -m 0755 target/release/funcspec "$(INSTALL_DIR)/funcspec"
	@echo "Installed funcspec -> $(INSTALL_DIR)/funcspec"
	@"$(INSTALL_DIR)/funcspec" --version 2>/dev/null || true

## Run all tests
test:
	cargo test --all

## Run clippy lints (deny warnings)
clippy:
	cargo clippy --all-targets --all-features -- -D warnings

## Format all code
fmt:
	cargo fmt --all

## Check formatting without modifying files
fmt-check:
	cargo fmt --all -- --check

## Run fmt-check + clippy + test
check: fmt-check clippy test

## Remove build artifacts
clean:
	cargo clean

## Show this help
help:
	@echo "Available targets:"
	@grep -E '^## ' Makefile | sed 's/^## /  /' | cat
	@echo ""
	@grep -E '^[a-z_-]+:' Makefile | sed 's/:.*//' | awk '{print "  make " $$1}'
