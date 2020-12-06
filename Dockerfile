FROM rust:1.47.0
WORKDIR /usr/src

RUN USER=root cargo new reddsaver

WORKDIR /usr/src/reddsaver
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

COPY src ./src
RUN cargo build --release
RUN mkdir -pv /app
RUN cp ./target/release/reddsaver /app/reddsaver

WORKDIR /app
CMD ["./reddsaver"]
