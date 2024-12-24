FROM docker.io/rust:1.83.0-bookworm as build

WORKDIR /usr/src/myapp
COPY . .
RUN cargo build --release

FROM docker.io/debian:bookworm-slim as runtime
RUN apt update -y && apt install libsqlite3-0 libssl3 ca-certificates -y && update-ca-certificates

FROM runtime as base
WORKDIR /app
EXPOSE 3000
ENV RUST_LOG=info
COPY --from=build /usr/src/myapp/target/release/danmakuhub /app

CMD ["./danmakuhub"]
