# Deployment Packaging

## Scope

This repository now ships a first-class packaging layer for containerized operation:
- Docker image (`Dockerfile`)
- Local composition (`docker-compose.yml`)
- Kubernetes baseline (`client/runtime/deploy/k8s/`)

The packaging gate is machine-checked by:

```bash
node client/runtime/systems/ops/deployment_packaging.ts run --profile=prod --strict=1
```

## Docker

Build:

```bash
docker build -t infring:local .
```

Run once:

```bash
docker run --rm \
  -e CLEARANCE=3 \
  -v "$(pwd)/state:/app/state" \
  -v "$(pwd)/logs:/app/logs" \
  -v "$(pwd)/secrets:/app/secrets:ro" \
  infring:local

# Smoke endpoints
curl -fsS http://127.0.0.1:4173/healthz
curl -fsS http://127.0.0.1:4173/dashboard >/dev/null
```

## Compose

```bash
docker compose up --build
```

## Kubernetes

```bash
kubectl apply -f client/runtime/deploy/k8s/namespace.yaml
kubectl apply -f client/runtime/deploy/k8s/configmap.yaml
kubectl apply -f client/runtime/deploy/k8s/networkpolicy.yaml
kubectl apply -f client/runtime/deploy/k8s/cronjob-daily.yaml
```

Notes:
- Cron cadence defaults to every 4 hours.
- Security defaults enforce non-root, no privilege escalation, and read-only root filesystem in the cron workload.
- Replace `emptyDir` volumes with PVCs for persistent state/log retention.

## Multi-Tenant Deployment

- Tenant state roots are isolated by namespace policy and deny-by-default cross-namespace reads.
- Per-tenant state roots are required for runtime and core control-plane surfaces.

## RBAC/ABAC

- RBAC operations are tenant scoped and require MFA in enterprise access policy.
- ABAC defaults to deny and records immutable hash-chained policy flight-recorder evidence.

## KMS-backed Secret Handling

- Secret handling uses opaque handles only; plaintext material is not exported through runtime APIs.
- CMEK routing is policy-bound (`kms://...`) and tied to zero-trust profile controls.

## Signed Receipts

- Regulated exports require signed receipt chains (`hmac_sha256`) and required signing env binding.
- Missing signing material is treated as a gate failure under strict readiness checks.

## Retention Policy Packs

- Runtime retention and compliance retention policies are bundled as a single policy pack.
- Pack validation requires active runtime targets plus hot/warm/cold compliance tiers.

## Exportable Audit Trails

- Audit export policy must provide latest/history output paths.
- Evidence audit policy must provide export json/markdown paths and receipt log path.
