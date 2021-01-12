FROM rust:1.45.0
WORKDIR /app

RUN cargo install diesel_cli --no-default-features --features postgres

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

COPY auth/Cargo.toml auth/Cargo.toml
RUN mkdir -p auth/src
RUN touch auth/src/lib.rs

COPY db/Cargo.toml db/Cargo.toml
RUN mkdir -p db/src
RUN touch db/src/lib.rs

COPY errors/Cargo.toml errors/Cargo.toml
RUN mkdir -p errors/src
RUN touch errors/src/lib.rs

COPY seeds/Cargo.toml seeds/Cargo.toml
RUN mkdir -p seeds/src
RUN echo "fn main() {}" > seeds/src/main.rs

COPY server/Cargo.toml server/Cargo.toml
RUN mkdir -p server/src
RUN echo "fn main() {}" > server/src/main.rs

RUN cargo build

COPY . .

RUN cargo build --bin server
