FROM node:22-bookworm AS dashboard
WORKDIR /src/cloud/dashboard
COPY cloud/dashboard/package*.json ./
RUN npm install
COPY cloud/dashboard ./
RUN npm run build

FROM rust:1-bookworm AS rust-builder
WORKDIR /src
COPY . .
RUN cargo build --release -p vsn-control-plane

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=rust-builder /src/target/release/vsn-control-plane /usr/local/bin/vsn-control-plane
COPY --from=dashboard /src/cloud/dashboard/dist /app/dashboard
ENV VSN_CONTROL_BIND=0.0.0.0:9070
ENV VSN_DASHBOARD_DIR=/app/dashboard
EXPOSE 9070
ENTRYPOINT ["vsn-control-plane"]
