FROM node:24.19.0-alpine@sha256:d32cdf619f63fe0471182d08996dd516c6275bb5fd31ae06e55a570bd9e1ad43 AS ui
WORKDIR /ui
COPY Corvus/apps/ui/package*.json ./
RUN npm ci
COPY Corvus/apps/ui/ ./
RUN npm run build -- --base-href /corvus/

FROM rust:1.85.1-bookworm AS api
WORKDIR /workspace/Corvus
COPY Tordo.SDK/rust/ /workspace/Tordo.SDK/rust/
COPY Corvus/Cargo.toml Corvus/Cargo.lock Corvus/rust-toolchain.toml ./
COPY Corvus/.cargo/ ./.cargo/
COPY Corvus/apps/api/ ./apps/api/
RUN cargo build --locked --release --bin corvus-api

FROM ubuntu:26.04@sha256:6a06111c3b0a8600feadb270aec771dc239bf407b25625f93f44ab00e11f4d3c
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libgcc-s1 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=api /workspace/Corvus/target/release/corvus-api /app/api
COPY --from=ui /ui/dist/corvus-ui/browser/ /app/ui/
ENV UI_DIR=/app/ui
USER 1000:1000
ENTRYPOINT ["/app/api"]
