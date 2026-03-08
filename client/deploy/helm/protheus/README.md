# Protheus Helm Chart

This chart deploys the scheduled Protheus spine workload with hardened defaults.
It also supports an always-on daemon deployment profile for multi-node operations.

## Install

```bash
helm upgrade --install protheus ./client/deploy/helm/protheus --namespace protheus --create-namespace
```

If runtime credentials are required, reference an existing secret:

```bash
helm upgrade --install protheus ./client/deploy/helm/protheus \
  --namespace protheus \
  --create-namespace \
  --set secrets.existingSecretName=protheus-runtime-secrets
```

## Key Values

- `image.repository`, `image.tag`: runtime image source
- `cron.schedule`: cadence for the daily spine run
- `daemon.enabled`, `daemon.replicas`: ambient daemon deployment mode
- `autoscaling.*`: optional HPA scaling contract for daemon workloads
- `runtimeConfig`: environment contract injected via ConfigMap
- `sso.*`: OIDC/SSO runtime env projection
- `secrets.existingSecretName`: optional runtime secret reference for credentials
- `secrets.create`: optional chart-managed secret creation from `secrets.data`
- `secrets.vault.*`, `secrets.kms.*`: secret provider integration metadata
- `nvidia.*`: GPU runtime class and adapter options
- `scheduling.*`: node selectors, tolerations, affinity for multi-node placement
- `networkPolicy.enabled`: deny-by-default ingress + controlled egress
- `conformance.enabled`: installs a Helm test pod (`helm test`) validating env wiring
