FROM rustlang/rust:nightly-slim AS builder

WORKDIR /crawler

COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

COPY --from=builder /crawler/target/release/crawler .

CMD ["./crawler"]
