# Multi-stage Dockerfile for CGMiner-RS
# High-performance ASIC Bitcoin miner written in Rust

# Build stage
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    gcc-aarch64-linux-gnu \
    gcc-arm-linux-gnueabihf \
    libc6-dev-arm64-cross \
    libc6-dev-armhf-cross \
    && rm -rf /var/lib/apt/lists/*

# Set up cross-compilation
RUN rustup target add x86_64-unknown-linux-gnu
RUN rustup target add aarch64-unknown-linux-gnu
RUN rustup target add armv7-unknown-linux-gnueabihf

# Create app directory
WORKDIR /app

# Copy dependency files
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./

# Create dummy source to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy source code
COPY src ./src
COPY drivers ./drivers
COPY config.toml ./

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim as runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create cgminer user
RUN groupadd -r cgminer && useradd -r -g cgminer cgminer

# Create directories
RUN mkdir -p /etc/cgminer-rs /var/log/cgminer-rs /var/lib/cgminer-rs
RUN chown -R cgminer:cgminer /var/log/cgminer-rs /var/lib/cgminer-rs

# Copy binary from builder stage
COPY --from=builder /app/target/release/cgminer-rs /usr/local/bin/cgminer-rs
COPY --from=builder /app/config.toml /etc/cgminer-rs/config.toml

# Set permissions
RUN chmod +x /usr/local/bin/cgminer-rs
RUN chown cgminer:cgminer /etc/cgminer-rs/config.toml

# Expose API port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/v1/status || exit 1

# Switch to cgminer user
USER cgminer

# Set working directory
WORKDIR /var/lib/cgminer-rs

# Default command
CMD ["cgminer-rs", "--config", "/etc/cgminer-rs/config.toml"]

# Development stage
FROM builder as development

# Install development tools
RUN apt-get update && apt-get install -y \
    gdb \
    valgrind \
    strace \
    curl \
    jq \
    && rm -rf /var/lib/apt/lists/*

# Install Rust development tools
RUN cargo install cargo-watch cargo-audit cargo-tarpaulin

# Set up development environment
WORKDIR /app
COPY . .

# Expose API port and debug port
EXPOSE 8080 9229

# Development command
CMD ["cargo", "watch", "-x", "run"]

# Cross-compilation stage for ARM64
FROM builder as arm64-builder

# Set cross-compilation environment
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
ENV CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
ENV CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++

# Build for ARM64
RUN cargo build --release --target aarch64-unknown-linux-gnu

# Cross-compilation stage for ARMv7
FROM builder as armv7-builder

# Set cross-compilation environment
ENV CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc
ENV CC_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-gcc
ENV CXX_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-g++

# Build for ARMv7
RUN cargo build --release --target armv7-unknown-linux-gnueabihf

# ARM64 runtime stage
FROM arm64v8/debian:bookworm-slim as arm64-runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create cgminer user
RUN groupadd -r cgminer && useradd -r -g cgminer cgminer

# Create directories
RUN mkdir -p /etc/cgminer-rs /var/log/cgminer-rs /var/lib/cgminer-rs
RUN chown -R cgminer:cgminer /var/log/cgminer-rs /var/lib/cgminer-rs

# Copy ARM64 binary
COPY --from=arm64-builder /app/target/aarch64-unknown-linux-gnu/release/cgminer-rs /usr/local/bin/cgminer-rs
COPY --from=arm64-builder /app/config.toml /etc/cgminer-rs/config.toml

# Set permissions
RUN chmod +x /usr/local/bin/cgminer-rs
RUN chown cgminer:cgminer /etc/cgminer-rs/config.toml

# Expose API port
EXPOSE 8080

# Switch to cgminer user
USER cgminer

# Set working directory
WORKDIR /var/lib/cgminer-rs

# Default command
CMD ["cgminer-rs", "--config", "/etc/cgminer-rs/config.toml"]

# ARMv7 runtime stage
FROM arm32v7/debian:bookworm-slim as armv7-runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create cgminer user
RUN groupadd -r cgminer && useradd -r -g cgminer cgminer

# Create directories
RUN mkdir -p /etc/cgminer-rs /var/log/cgminer-rs /var/lib/cgminer-rs
RUN chown -R cgminer:cgminer /var/log/cgminer-rs /var/lib/cgminer-rs

# Copy ARMv7 binary
COPY --from=armv7-builder /app/target/armv7-unknown-linux-gnueabihf/release/cgminer-rs /usr/local/bin/cgminer-rs
COPY --from=armv7-builder /app/config.toml /etc/cgminer-rs/config.toml

# Set permissions
RUN chmod +x /usr/local/bin/cgminer-rs
RUN chown cgminer:cgminer /etc/cgminer-rs/config.toml

# Expose API port
EXPOSE 8080

# Switch to cgminer user
USER cgminer

# Set working directory
WORKDIR /var/lib/cgminer-rs

# Default command
CMD ["cgminer-rs", "--config", "/etc/cgminer-rs/config.toml"]
