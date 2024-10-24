FROM rust:1.70.0-slim-bullseye AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
COPY --from=builder /app/target/release/server /usr/local/bin/
EXPOSE 80
CMD ["server"]
