FROM node:22-alpine AS deps
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --include=dev && npm cache clean --force

FROM node:22-alpine AS runtime
WORKDIR /app

COPY --from=deps /app/node_modules ./node_modules
COPY . .

ARG PROTHEUS_FIPS_MODE=0
ARG VCS_REF=unknown
ARG BUILD_DATE=unknown

RUN addgroup -S protheus && adduser -S protheus -G protheus \
  && mkdir -p /app/state /app/tmp /app/logs /app/secrets \
  && chown -R protheus:protheus /app \
  && test "$PROTHEUS_FIPS_MODE" = "0" -o "$PROTHEUS_FIPS_MODE" = "1"

ENV NODE_ENV=production
ENV CLEARANCE=3
ENV TZ=UTC
ENV PROTHEUS_FIPS_MODE=${PROTHEUS_FIPS_MODE}

LABEL org.opencontainers.image.title="protheus" \
      org.opencontainers.image.description="Protheus runtime image" \
      org.opencontainers.image.revision="${VCS_REF}" \
      org.opencontainers.image.created="${BUILD_DATE}" \
      org.opencontainers.image.vendor="protheuslabs" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.base.name="node:22-alpine"

USER protheus

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
  CMD node client/systems/autonomy/health_status.js >/dev/null || exit 1

CMD ["node", "client/systems/spine/spine.js", "daily"]
