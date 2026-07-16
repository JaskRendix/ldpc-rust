# ---- Build stage ----
FROM rust:latest AS builder

WORKDIR /app

# Copy manifest first (better caching)
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build only the server binary
RUN cargo build --release --bin server

# ---- Runtime stage ----
FROM debian:stable-slim

WORKDIR /app

# Copy the compiled binary
COPY --from=builder /app/target/release/server /app/server

EXPOSE 8080

CMD ["/app/server"]
