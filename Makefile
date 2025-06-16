# CGMiner-RS Makefile
# High-performance ASIC Bitcoin miner written in Rust

# Variables
CARGO = cargo
TARGET_DIR = target
RELEASE_DIR = $(TARGET_DIR)/release
DEBUG_DIR = $(TARGET_DIR)/debug
BINARY_NAME = cgminer-rs
CONFIG_FILE = config.toml
LOG_LEVEL = info

# Build targets
TARGETS = x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu armv7-unknown-linux-gnueabihf

# Default target
.PHONY: all
all: build

# Help target
.PHONY: help
help:
	@echo "CGMiner-RS Build System"
	@echo ""
	@echo "Available targets:"
	@echo "  build          - Build the project in debug mode"
	@echo "  release        - Build the project in release mode"
	@echo "  test           - Run all tests"
	@echo "  bench          - Run benchmarks"
	@echo "  clean          - Clean build artifacts"
	@echo "  install        - Install the binary"
	@echo "  uninstall      - Uninstall the binary"
	@echo "  cross-compile  - Cross-compile for all supported targets"
	@echo "  docker-build   - Build using Docker"
	@echo "  package        - Create distribution packages"
	@echo "  docs           - Generate documentation"
	@echo "  lint           - Run linting checks"
	@echo "  format         - Format code"
	@echo "  check          - Check code without building"
	@echo "  run            - Run the application"
	@echo "  dev            - Run in development mode with auto-reload"
	@echo ""

# Build targets
.PHONY: build
build:
	@echo "Building CGMiner-RS (debug)..."
	$(CARGO) build

.PHONY: release
release:
	@echo "Building CGMiner-RS (release)..."
	$(CARGO) build --release

.PHONY: check
check:
	@echo "Checking code..."
	$(CARGO) check
	$(CARGO) clippy -- -D warnings

# Test targets
.PHONY: test
test:
	@echo "Running tests..."
	$(CARGO) test

.PHONY: test-integration
test-integration:
	@echo "Running integration tests..."
	$(CARGO) test --test integration_tests

.PHONY: test-unit
test-unit:
	@echo "Running unit tests..."
	$(CARGO) test --lib

.PHONY: test-coverage
test-coverage:
	@echo "Running tests with coverage..."
	$(CARGO) tarpaulin --out Html --output-dir coverage

# Benchmark targets
.PHONY: bench
bench:
	@echo "Running benchmarks..."
	$(CARGO) bench

.PHONY: bench-baseline
bench-baseline:
	@echo "Running baseline benchmarks..."
	$(CARGO) bench -- --save-baseline baseline

.PHONY: bench-compare
bench-compare:
	@echo "Comparing benchmarks..."
	$(CARGO) bench -- --baseline baseline

# Code quality targets
.PHONY: lint
lint:
	@echo "Running linting checks..."
	$(CARGO) clippy -- -D warnings
	$(CARGO) fmt -- --check

.PHONY: format
format:
	@echo "Formatting code..."
	$(CARGO) fmt

.PHONY: audit
audit:
	@echo "Auditing dependencies..."
	$(CARGO) audit

# Documentation targets
.PHONY: docs
docs:
	@echo "Generating documentation..."
	$(CARGO) doc --no-deps --open

.PHONY: docs-private
docs-private:
	@echo "Generating documentation (including private items)..."
	$(CARGO) doc --no-deps --document-private-items --open

# Cross-compilation targets
.PHONY: cross-compile
cross-compile:
	@echo "Cross-compiling for all targets..."
	@for target in $(TARGETS); do \
		echo "Building for $$target..."; \
		$(CARGO) build --release --target $$target || echo "Failed to build for $$target"; \
	done

.PHONY: cross-compile-aarch64
cross-compile-aarch64:
	@echo "Cross-compiling for aarch64..."
	$(CARGO) build --release --target aarch64-unknown-linux-gnu

.PHONY: cross-compile-armv7
cross-compile-armv7:
	@echo "Cross-compiling for armv7..."
	$(CARGO) build --release --target armv7-unknown-linux-gnueabihf

# Docker targets
.PHONY: docker-build
docker-build:
	@echo "Building Docker image..."
	docker build -t cgminer-rs .

.PHONY: docker-run
docker-run:
	@echo "Running Docker container..."
	docker run --rm -it -p 8080:8080 cgminer-rs

.PHONY: docker-cross
docker-cross:
	@echo "Cross-compiling using Docker..."
	docker run --rm -v $(PWD):/workspace -w /workspace \
		rust:latest bash -c "rustup target add $(TARGETS) && make cross-compile"

# Installation targets
.PHONY: install
install: release
	@echo "Installing CGMiner-RS..."
	sudo cp $(RELEASE_DIR)/$(BINARY_NAME) /usr/local/bin/
	sudo chmod +x /usr/local/bin/$(BINARY_NAME)
	@if [ ! -f /etc/cgminer-rs/config.toml ]; then \
		sudo mkdir -p /etc/cgminer-rs; \
		sudo cp $(CONFIG_FILE) /etc/cgminer-rs/; \
	fi
	@echo "CGMiner-RS installed successfully!"

.PHONY: uninstall
uninstall:
	@echo "Uninstalling CGMiner-RS..."
	sudo rm -f /usr/local/bin/$(BINARY_NAME)
	@echo "CGMiner-RS uninstalled successfully!"

# Package targets
.PHONY: package
package: release
	@echo "Creating distribution packages..."
	mkdir -p dist
	
	# Create tarball
	tar -czf dist/cgminer-rs-$(shell $(CARGO) metadata --format-version 1 | jq -r '.packages[0].version')-linux-x86_64.tar.gz \
		-C $(RELEASE_DIR) $(BINARY_NAME) \
		-C ../../ $(CONFIG_FILE) README.md LICENSE
	
	# Create DEB package (requires fpm)
	@if command -v fpm >/dev/null 2>&1; then \
		fpm -s dir -t deb -n cgminer-rs \
			--version $(shell $(CARGO) metadata --format-version 1 | jq -r '.packages[0].version') \
			--description "High-performance ASIC Bitcoin miner" \
			--maintainer "CGMiner-RS Team" \
			--license "MIT" \
			--url "https://github.com/cgminer-rs/cgminer-rs" \
			--package dist/ \
			$(RELEASE_DIR)/$(BINARY_NAME)=/usr/local/bin/$(BINARY_NAME) \
			$(CONFIG_FILE)=/etc/cgminer-rs/config.toml; \
	else \
		echo "fpm not found, skipping DEB package creation"; \
	fi

# Development targets
.PHONY: run
run: build
	@echo "Running CGMiner-RS..."
	RUST_LOG=$(LOG_LEVEL) $(DEBUG_DIR)/$(BINARY_NAME) --config $(CONFIG_FILE)

.PHONY: run-release
run-release: release
	@echo "Running CGMiner-RS (release)..."
	RUST_LOG=$(LOG_LEVEL) $(RELEASE_DIR)/$(BINARY_NAME) --config $(CONFIG_FILE)

.PHONY: dev
dev:
	@echo "Running in development mode with auto-reload..."
	$(CARGO) watch -x 'run -- --config $(CONFIG_FILE)'

# Utility targets
.PHONY: clean
clean:
	@echo "Cleaning build artifacts..."
	$(CARGO) clean
	rm -rf dist/
	rm -rf coverage/

.PHONY: clean-all
clean-all: clean
	@echo "Cleaning all artifacts including dependencies..."
	rm -rf target/
	rm -rf Cargo.lock

.PHONY: update
update:
	@echo "Updating dependencies..."
	$(CARGO) update

.PHONY: tree
tree:
	@echo "Dependency tree..."
	$(CARGO) tree

.PHONY: bloat
bloat: release
	@echo "Analyzing binary size..."
	$(CARGO) bloat --release

.PHONY: size
size: release
	@echo "Binary size analysis..."
	ls -lh $(RELEASE_DIR)/$(BINARY_NAME)
	@if command -v strip >/dev/null 2>&1; then \
		cp $(RELEASE_DIR)/$(BINARY_NAME) $(RELEASE_DIR)/$(BINARY_NAME).stripped; \
		strip $(RELEASE_DIR)/$(BINARY_NAME).stripped; \
		echo "Stripped size:"; \
		ls -lh $(RELEASE_DIR)/$(BINARY_NAME).stripped; \
	fi

# CI/CD targets
.PHONY: ci
ci: check test lint audit

.PHONY: ci-full
ci-full: ci bench docs cross-compile

# Performance targets
.PHONY: profile
profile: release
	@echo "Profiling application..."
	perf record --call-graph=dwarf $(RELEASE_DIR)/$(BINARY_NAME) --config $(CONFIG_FILE)
	perf report

.PHONY: flamegraph
flamegraph: release
	@echo "Generating flamegraph..."
	$(CARGO) flamegraph --bin $(BINARY_NAME) -- --config $(CONFIG_FILE)

# Debug targets
.PHONY: debug
debug: build
	@echo "Running with debugger..."
	gdb $(DEBUG_DIR)/$(BINARY_NAME)

.PHONY: valgrind
valgrind: build
	@echo "Running with valgrind..."
	valgrind --tool=memcheck --leak-check=full $(DEBUG_DIR)/$(BINARY_NAME) --config $(CONFIG_FILE)

# Configuration targets
.PHONY: config-check
config-check:
	@echo "Validating configuration..."
	$(DEBUG_DIR)/$(BINARY_NAME) --config $(CONFIG_FILE) --check-config

.PHONY: config-example
config-example:
	@echo "Generating example configuration..."
	$(DEBUG_DIR)/$(BINARY_NAME) --generate-config > config.example.toml

# Version information
.PHONY: version
version:
	@echo "CGMiner-RS version information:"
	@$(CARGO) metadata --format-version 1 | jq -r '.packages[0].version'
	@echo "Rust version: $(shell rustc --version)"
	@echo "Cargo version: $(shell cargo --version)"

# Setup development environment
.PHONY: setup-dev
setup-dev:
	@echo "Setting up development environment..."
	rustup component add clippy rustfmt
	rustup target add $(TARGETS)
	@if ! command -v cargo-watch >/dev/null 2>&1; then \
		cargo install cargo-watch; \
	fi
	@if ! command -v cargo-audit >/dev/null 2>&1; then \
		cargo install cargo-audit; \
	fi
	@if ! command -v cargo-tarpaulin >/dev/null 2>&1; then \
		cargo install cargo-tarpaulin; \
	fi
	@echo "Development environment setup complete!"

# Show build information
.PHONY: info
info:
	@echo "Build Information:"
	@echo "  Project: CGMiner-RS"
	@echo "  Version: $(shell $(CARGO) metadata --format-version 1 | jq -r '.packages[0].version')"
	@echo "  Target Directory: $(TARGET_DIR)"
	@echo "  Supported Targets: $(TARGETS)"
	@echo "  Rust Version: $(shell rustc --version)"
	@echo "  Cargo Version: $(shell cargo --version)"
