# Protheus Helm Chart

This chart deploys the scheduled Protheus spine workload with hardened defaults.

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
- `runtimeConfig`: environment contract injected via ConfigMap
- `secrets.existingSecretName`: optional runtime secret reference for credentials
- `secrets.create`: optional chart-managed secret creation from `secrets.data`
- `networkPolicy.enabled`: deny-by-default ingress + controlled egress
