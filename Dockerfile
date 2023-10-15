FROM rust:1.73.0-buster as build

WORKDIR /usr/src/myapp
COPY . .
RUN cargo build --release

FROM debian:buster-slim as runtime
RUN apt update -y && apt install libsqlite3-0 libssl1.1 -y

FROM runtime as base
WORKDIR /app
EXPOSE 3000
ENV RUST_LOG=info
COPY --from=build /usr/src/myapp/target/release/danmakuhub /app

CMD ["./danmakuhub"]
