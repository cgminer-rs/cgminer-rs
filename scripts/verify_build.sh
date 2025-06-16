#!/bin/bash

# CGMiner-RS Build Verification Script
# This script performs comprehensive verification of the build system

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
TESTS_PASSED=0
TESTS_FAILED=0
TOTAL_TESTS=0

# Logging functions
log() {
    echo -e "${BLUE}[$(date +'%H:%M:%S')]${NC} $1"
}

success() {
    echo -e "${GREEN}✓${NC} $1"
    ((TESTS_PASSED++))
}

error() {
    echo -e "${RED}✗${NC} $1"
    ((TESTS_FAILED++))
}

warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

run_test() {
    local test_name="$1"
    local test_command="$2"
    
    ((TOTAL_TESTS++))
    log "Running: $test_name"
    
    if eval "$test_command" > /dev/null 2>&1; then
        success "$test_name"
        return 0
    else
        error "$test_name"
        return 1
    fi
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."
    
    run_test "Rust compiler available" "rustc --version"
    run_test "Cargo available" "cargo --version"
    run_test "Git available" "git --version"
    
    # Check Rust version
    local rust_version=$(rustc --version | awk '{print $2}')
    local required_version="1.75.0"
    
    if [[ "$(printf '%s\n' "$required_version" "$rust_version" | sort -V | head -n1)" == "$required_version" ]]; then
        success "Rust version $rust_version >= $required_version"
    else
        error "Rust version $rust_version < $required_version"
    fi
}

# Check project structure
check_project_structure() {
    log "Checking project structure..."
    
    local required_files=(
        "Cargo.toml"
        "build.rs"
        "src/main.rs"
        "src/config.rs"
        "src/device/mod.rs"
        "src/mining/mod.rs"
        "src/pool/mod.rs"
        "src/api/mod.rs"
        "src/monitoring/mod.rs"
        "src/error.rs"
        "src/ffi.rs"
        "config.toml"
        "README.md"
        "LICENSE"
        "Makefile"
        "Dockerfile"
        "docker-compose.yml"
    )
    
    for file in "${required_files[@]}"; do
        if [[ -f "$file" ]]; then
            success "File exists: $file"
        else
            error "Missing file: $file"
        fi
    done
    
    local required_dirs=(
        "src/device"
        "src/mining"
        "src/pool"
        "src/api"
        "src/monitoring"
        "drivers"
        "tests"
        "benches"
        "docs"
        "examples"
        "scripts"
    )
    
    for dir in "${required_dirs[@]}"; do
        if [[ -d "$dir" ]]; then
            success "Directory exists: $dir"
        else
            error "Missing directory: $dir"
        fi
    done
}

# Check dependencies
check_dependencies() {
    log "Checking dependencies..."
    
    run_test "Cargo check" "cargo check"
    run_test "Dependency tree" "cargo tree --depth 1"
    
    # Check for security vulnerabilities
    if command -v cargo-audit >/dev/null 2>&1; then
        run_test "Security audit" "cargo audit"
    else
        warning "cargo-audit not installed, skipping security check"
    fi
}

# Build tests
build_tests() {
    log "Running build tests..."
    
    run_test "Debug build" "cargo build"
    run_test "Release build" "cargo build --release"
    run_test "Documentation build" "cargo doc --no-deps"
    
    # Check binary exists
    if [[ -f "target/debug/cgminer-rs" ]]; then
        success "Debug binary created"
    else
        error "Debug binary not found"
    fi
    
    if [[ -f "target/release/cgminer-rs" ]]; then
        success "Release binary created"
    else
        error "Release binary not found"
    fi
}

# Code quality tests
code_quality_tests() {
    log "Running code quality tests..."
    
    run_test "Code formatting" "cargo fmt -- --check"
    run_test "Clippy lints" "cargo clippy -- -D warnings"
    
    # Check for TODO/FIXME comments
    local todo_count=$(grep -r "TODO\|FIXME" src/ || true | wc -l)
    if [[ $todo_count -eq 0 ]]; then
        success "No TODO/FIXME comments found"
    else
        warning "$todo_count TODO/FIXME comments found"
    fi
}

# Unit tests
unit_tests() {
    log "Running unit tests..."
    
    run_test "Unit tests" "cargo test --lib"
    run_test "Integration tests" "cargo test --test integration_tests"
    
    # Run with coverage if available
    if command -v cargo-tarpaulin >/dev/null 2>&1; then
        run_test "Test coverage" "cargo tarpaulin --out Xml --output-dir coverage"
    else
        warning "cargo-tarpaulin not installed, skipping coverage"
    fi
}

# Benchmark tests
benchmark_tests() {
    log "Running benchmark tests..."
    
    if [[ -d "benches" ]]; then
        run_test "Benchmarks" "cargo bench --bench mining_benchmark"
    else
        warning "No benchmarks directory found"
    fi
}

# Configuration tests
config_tests() {
    log "Testing configuration..."
    
    # Test example configurations
    local config_files=(
        "config.toml"
        "examples/basic_config.toml"
        "examples/advanced_config.toml"
    )
    
    for config in "${config_files[@]}"; do
        if [[ -f "$config" ]]; then
            # Basic TOML syntax check
            if command -v toml >/dev/null 2>&1; then
                run_test "TOML syntax: $config" "toml get $config . >/dev/null"
            else
                success "Config file exists: $config"
            fi
        else
            error "Missing config file: $config"
        fi
    done
}

# Docker tests
docker_tests() {
    log "Testing Docker build..."
    
    if command -v docker >/dev/null 2>&1; then
        run_test "Docker build" "docker build -t cgminer-rs-test ."
        
        # Clean up test image
        docker rmi cgminer-rs-test >/dev/null 2>&1 || true
    else
        warning "Docker not available, skipping Docker tests"
    fi
}

# Cross-compilation tests
cross_compilation_tests() {
    log "Testing cross-compilation..."
    
    # Check if cross-compilation targets are installed
    local targets=("aarch64-unknown-linux-gnu" "armv7-unknown-linux-gnueabihf")
    
    for target in "${targets[@]}"; do
        if rustup target list --installed | grep -q "$target"; then
            run_test "Cross-compile for $target" "cargo build --target $target"
        else
            warning "Target $target not installed, skipping"
        fi
    done
}

# Performance tests
performance_tests() {
    log "Running performance tests..."
    
    # Check binary size
    if [[ -f "target/release/cgminer-rs" ]]; then
        local size=$(stat -c%s "target/release/cgminer-rs" 2>/dev/null || stat -f%z "target/release/cgminer-rs" 2>/dev/null || echo "0")
        local size_mb=$((size / 1024 / 1024))
        
        if [[ $size_mb -lt 50 ]]; then
            success "Binary size acceptable: ${size_mb}MB"
        else
            warning "Binary size large: ${size_mb}MB"
        fi
    fi
    
    # Check compilation time
    local start_time=$(date +%s)
    cargo build --release >/dev/null 2>&1
    local end_time=$(date +%s)
    local compile_time=$((end_time - start_time))
    
    if [[ $compile_time -lt 300 ]]; then  # 5 minutes
        success "Compilation time acceptable: ${compile_time}s"
    else
        warning "Compilation time slow: ${compile_time}s"
    fi
}

# Documentation tests
documentation_tests() {
    log "Testing documentation..."
    
    local doc_files=(
        "README.md"
        "CONTRIBUTING.md"
        "docs/API.md"
        "docs/CONFIGURATION.md"
    )
    
    for doc in "${doc_files[@]}"; do
        if [[ -f "$doc" ]]; then
            # Check if file is not empty
            if [[ -s "$doc" ]]; then
                success "Documentation exists: $doc"
            else
                error "Documentation empty: $doc"
            fi
        else
            error "Missing documentation: $doc"
        fi
    done
    
    # Check for broken links (basic check)
    if command -v grep >/dev/null 2>&1; then
        local broken_links=$(grep -r "http://localhost" docs/ || true | wc -l)
        if [[ $broken_links -eq 0 ]]; then
            success "No localhost links in documentation"
        else
            warning "$broken_links localhost links found in documentation"
        fi
    fi
}

# Security tests
security_tests() {
    log "Running security tests..."
    
    # Check for hardcoded secrets
    local secret_patterns=("password" "secret" "key" "token")
    local secrets_found=0
    
    for pattern in "${secret_patterns[@]}"; do
        local count=$(grep -ri "$pattern.*=" src/ | grep -v "test\|example\|placeholder" | wc -l || echo "0")
        secrets_found=$((secrets_found + count))
    done
    
    if [[ $secrets_found -eq 0 ]]; then
        success "No hardcoded secrets found"
    else
        warning "$secrets_found potential hardcoded secrets found"
    fi
    
    # Check file permissions
    local executable_files=$(find src/ -type f -executable | wc -l)
    if [[ $executable_files -eq 0 ]]; then
        success "No executable source files"
    else
        warning "$executable_files executable source files found"
    fi
}

# Generate report
generate_report() {
    log "Generating verification report..."
    
    echo
    echo "=================================="
    echo "CGMiner-RS Build Verification Report"
    echo "=================================="
    echo "Total Tests: $TOTAL_TESTS"
    echo "Passed: $TESTS_PASSED"
    echo "Failed: $TESTS_FAILED"
    echo
    
    if [[ $TESTS_FAILED -eq 0 ]]; then
        echo -e "${GREEN}🎉 All tests passed! Build verification successful.${NC}"
        echo
        echo "The project is ready for:"
        echo "- Development"
        echo "- Testing"
        echo "- Deployment"
        echo "- Distribution"
        return 0
    else
        echo -e "${RED}❌ $TESTS_FAILED tests failed. Please review and fix issues.${NC}"
        echo
        echo "Common fixes:"
        echo "- Run 'make setup-dev' to install development tools"
        echo "- Run 'cargo fmt' to fix formatting"
        echo "- Run 'cargo clippy --fix' to fix linting issues"
        echo "- Check missing files and directories"
        return 1
    fi
}

# Main execution
main() {
    echo "CGMiner-RS Build Verification"
    echo "============================="
    echo
    
    # Change to project root if script is run from scripts directory
    if [[ "$(basename "$PWD")" == "scripts" ]]; then
        cd ..
    fi
    
    # Run all test suites
    check_prerequisites
    check_project_structure
    check_dependencies
    build_tests
    code_quality_tests
    unit_tests
    benchmark_tests
    config_tests
    docker_tests
    cross_compilation_tests
    performance_tests
    documentation_tests
    security_tests
    
    # Generate final report
    generate_report
}

# Run main function
main "$@"
