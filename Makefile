# Amberjs Build System
# JavaScript/TypeScript runtime built with Rust and V8

.PHONY: all build test test-bundle-contract clean run help install dev release

# Default target
all: build test

# Build the project
build:
	@echo "Building Amberjs..."
	cargo build --release --bin amber

# Run tests
test:
	@echo "Running tests..."
	cargo test
	@echo "✅ All tests passed!"

# Run with specific file
run: build
	@echo "Running example..."
	./target/release/amber run $(file)

# Clean build artifacts
clean:
	@echo "Cleaning..."
	cargo clean

# Install to system
install: build
	@echo "Installing Amberjs..."
	sudo cp target/release/amber /usr/local/bin/

# Development build (faster)
dev: build

# Release build (optimized)
release: build

# Performance test
perf: build
	@echo "Running performance test..."
	./target/release/amber run examples/performance/performance_test.js

# Idle memory & HTTP throughput benchmark
bench-idle: build
	@echo "Running idle memory & throughput benchmark..."
	python3 benchmarks/idle_memory/runner.py --framework http --duration 5

# Hello world example
hello: build
	@echo "Running hello world example..."
	./target/release/amber run examples/basics/hello_world.js

# Pin the Stable amber bundle contract (docs/BUNDLE_CONTRACT.md)
test-bundle-contract:
	@echo "Running amber bundle contract tests..."
	cargo test --test bundle_contract_tests -- --test-threads=1

# Check formatting
fmt:
	@echo "Checking code formatting..."
	cargo fmt --all -- --check

# Lint code
lint:
	@echo "Linting code..."
	cargo clippy --all-targets -- -D warnings

# Show help
help:
	@echo "Amberjs - JavaScript/TypeScript runtime built with Rust and V8"
	@echo ""
	@echo "Available targets:"
	@echo "  build   - Build the project"
	@echo "  test    - Run all tests"
	@echo "  test-bundle-contract - Pin Stable amber bundle contract"
	@echo "  run     - Run with a specific file (use: make run file=script.js)"
	@echo "  clean   - Clean build artifacts"
	@echo "  install - Install to system"
	@echo "  dev     - Development build"
	@echo "  release - Release build"
	@echo "  perf        - Run performance test"
	@echo "  bench-idle  - Run idle memory & HTTP throughput benchmark"
	@echo "  hello       - Run hello world example"
	@echo "  fmt     - Check code formatting"
	@echo "  lint    - Lint code"
	@echo "  help    - Show this help"
