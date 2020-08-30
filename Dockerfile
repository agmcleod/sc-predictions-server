FROM rust:1.45.0
WORKDIR /app
COPY Cargo.toml .
COPY Cargo.lock .
RUN mkdir -p src
RUN echo 'fn main() {}' > src/main.rs

RUN cargo build --bin server

RUN cargo install diesel_cli --no-default-features --features postgres

# We need to touch our real main.rs file or else docker will use
# the cached one.
COPY . .
RUN touch src/main.rs

RUN cargo build --bin server
