# Build stage
FROM rust:1.82-slim as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Copy source code
COPY src ./src

# Build for release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/imagepreview /app/imagepreview

# Expose port (adjust if your app uses a different port)
EXPOSE 3000

# Run the binary
CMD ["/app/imagepreview"]
