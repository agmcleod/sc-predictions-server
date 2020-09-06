FROM rust:1.45.0
WORKDIR /app

RUN cargo install diesel_cli --no-default-features --features postgres

COPY . .

RUN cargo build --bin server
