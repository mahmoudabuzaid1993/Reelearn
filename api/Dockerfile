# Build context is the repository root (docker-compose sets it), so the migrations and the
# OpenAPI contract next to the crates are available:  docker build -f api/Dockerfile .
FROM rust:1-bookworm AS build
WORKDIR /src
COPY api/Cargo.toml api/Cargo.lock ./api/
COPY api/crates ./api/crates
COPY db ./db
RUN cd api && cargo build --release -p reelearn-api -p reelearn-worker

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /src/api/target/release/reelearn-api /src/api/target/release/reelearn-worker /usr/local/bin/
COPY openapi/openapi.yaml /app/openapi/openapi.yaml
ENV OPENAPI_PATH=/app/openapi/openapi.yaml PORT=8080
EXPOSE 8080
USER 65534:65534
# The same image runs the worker with:  CMD ["reelearn-worker"]
CMD ["reelearn-api"]
