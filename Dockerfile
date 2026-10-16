FROM rust:1-alpine AS chef
RUN apk add --no-cache build-base cmake
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS build
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY proto/ proto/
RUN cargo build --release --locked

FROM node:24-alpine AS web
RUN corepack enable pnpm
WORKDIR /web
COPY web/package.json web/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY web/ ./
RUN pnpm build

FROM alpine:3
COPY --from=web /web/dist /web/dist
COPY --from=build /app/target/release/serve /app/target/release/console /app/target/release/lookup-sync /app/target/release/alerting /app/target/release/uptime /
USER nobody
CMD ["/serve"]
