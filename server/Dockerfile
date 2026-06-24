# --- build stage ---
FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml ./
COPY Cargo.lock* ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release

# --- runtime stage (rustls everywhere → no openssl needed) ---
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/michael-backend /usr/local/bin/michael-backend
COPY migrations ./migrations
EXPOSE 8080
CMD ["michael-backend"]
