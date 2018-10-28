FROM rust:1.29.2
WORKDIR /app
COPY Cargo.toml .
COPY Cargo.lock .
RUN mkdir -p src
RUN echo 'fn main() {}' > src/main.rs

RUN cargo build

RUN cargo install dbmigrate --features postgres_support

# We need to touch our real main.rs file or else docker will use
# the cached one.
COPY . .
RUN touch src/main.rs

RUN cargo build
