FROM rust:1-alpine AS build
RUN apk add --no-cache build-base cmake protoc
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY proto/ proto/
RUN cargo build --release --locked

FROM alpine:3
COPY --from=build /app/target/release/serve /app/target/release/lookup-sync /app/target/release/alerting /
USER nobody
CMD ["/serve"]
