.PHONY: build test clippy fmt fmt-check release clean check

# Default target
all: fmt-check clippy test

## Build debug binary
build:
	cargo build

## Build optimized release binary
release:
	cargo build --release -p funcspec-cli
	@echo "Binary at: target/release/funcspec"

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
