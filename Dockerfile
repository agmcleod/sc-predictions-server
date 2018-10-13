FROM rust:1.29.2
WORKDIR /app
COPY . .

RUN cargo build
