# Stage 1: build
FROM rust:1.85-slim-bullseye AS builder

RUN apt-get update && apt-get install -y libssl-dev pkg-config git libpq-dev \
   && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace Cargo.toml and Cargo.lock
COPY Cargo.toml Cargo.lock ./

# Copy each crate's manifest from crates/…
COPY crates/common/Cargo.toml      common/
COPY crates/config_manager/Cargo.toml config_manager/
COPY crates/deploy_service/Cargo.toml deploy_service/
COPY crates/data_models/Cargo.toml data_models/
COPY crates/auth_captcha/Cargo.toml auth_captcha/
COPY crates/api_gateway/Cargo.toml   api_gateway/

# Copy the full source so we can build
COPY . .

# Build just the api_gateway binary in release
WORKDIR /app/crates/api_gateway
RUN cargo build --release --bin api_gateway

# Stage 2: runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl-dev pkg-config git libpq-dev \
   && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/api_gateway /usr/local/bin/api_gateway

# Copy shared config and tasks
COPY Config.toml /app/Config.toml
COPY tasks      /app/tasks

WORKDIR /app
EXPOSE 8080
ENTRYPOINT ["api_gateway"]
