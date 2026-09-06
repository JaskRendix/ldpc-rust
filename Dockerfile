# ---- Build stage ----
FROM rust:slim AS builder

WORKDIR /app

# Copy dependency manifests and source trees
COPY Cargo.toml ./
COPY Cargo.lock* ./
COPY src ./src
COPY benches ./benches
COPY tests ./tests

# Build only the server binary in release mode
RUN cargo build --release --bin server

# ---- Runtime stage ----
FROM debian:stable-slim

WORKDIR /app

# Create a non-root system user for security
RUN useradd -u 10001 -ms /bin/bash appuser

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/server /app/server

# Change ownership to the non-root user
RUN chown appuser:appuser /app/server
USER appuser

EXPOSE 8080

CMD ["/app/server"]
