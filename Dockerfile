FROM node:22-alpine AS deps
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --include=dev && npm cache clean --force

FROM rust:1.89-alpine AS rust-builder
WORKDIR /app
RUN apk add --no-cache build-base musl-dev pkgconfig openssl-dev
COPY . .
RUN cargo build --release --manifest-path core/layer0/ops/Cargo.toml --bin protheus-ops --bin protheusd

FROM node:22-alpine AS runtime
WORKDIR /app

COPY --from=deps /app/node_modules ./node_modules
COPY . .
COPY --from=rust-builder /app/target/release/protheus-ops /app/target/release/protheus-ops
COPY --from=rust-builder /app/target/release/protheusd /app/target/release/protheusd

ARG INFRING_FIPS_MODE=0
ARG VCS_REF=unknown
ARG BUILD_DATE=unknown

RUN addgroup -S infring && adduser -S infring -G infring \
  && mkdir -p /app/state /app/tmp /app/logs /app/secrets \
  && chown -R infring:infring /app \
  && test "$INFRING_FIPS_MODE" = "0" -o "$INFRING_FIPS_MODE" = "1"

ENV NODE_ENV=production
ENV CLEARANCE=3
ENV TZ=UTC
ENV INFRING_FIPS_MODE=${INFRING_FIPS_MODE}
ENV PROTHEUS_NPM_BINARY=/app/target/release/protheus-ops

LABEL org.opencontainers.image.title="infring" \
      org.opencontainers.image.description="InfRing runtime image" \
      org.opencontainers.image.revision="${VCS_REF}" \
      org.opencontainers.image.created="${BUILD_DATE}" \
      org.opencontainers.image.vendor="InfRing Project" \
      org.opencontainers.image.licenses="Apache-2.0 AND LicenseRef-InfRing-NC-1.0" \
      org.opencontainers.image.base.name="node:22-alpine"

USER infring

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
  CMD node -e "fetch('http://127.0.0.1:4173/healthz',{cache:'no-store'}).then((r)=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))"

EXPOSE 4173

CMD ["node", "client/runtime/lib/ts_entrypoint.ts", "client/runtime/systems/ui/infring_dashboard.ts", "serve", "--host=0.0.0.0", "--port=4173", "--team=ops", "--refresh-ms=2000"]
