# Stage 1: build
FROM rust:1.85-slim-bullseye AS builder

RUN apt-get update && apt-get install -y libssl-dev pkg-config git libpq5 \
   && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./

COPY crates/common/Cargo.toml           common/
COPY crates/config_manager/Cargo.toml  config_manager/
COPY crates/deploy_service/Cargo.toml  deploy_service/
COPY crates/data_models/Cargo.toml     data_models/
COPY crates/auth_captcha/Cargo.toml    auth_captcha/
COPY crates/scheduler_service/Cargo.toml scheduler_service/

COPY . .

WORKDIR /app/crates/scheduler_service
RUN cargo build --release --bin scheduler_service

# Stage 2: runtime
FROM debian:bookworm-slim
COPY --from=builder /app/target/release/scheduler_service /usr/local/bin/scheduler_service

COPY Config.toml /app/Config.toml
COPY tasks      /app/tasks

WORKDIR /app
ENTRYPOINT ["scheduler_service"]
