# Protheus Kubernetes Manifests

Apply order:

```bash
kubectl apply -f client/runtime/deploy/k8s/namespace.yaml
kubectl apply -f client/runtime/deploy/k8s/configmap.yaml
kubectl apply -f client/runtime/deploy/k8s/secret.runtime.example.yaml
kubectl apply -f client/runtime/deploy/k8s/networkpolicy.yaml
kubectl apply -f client/runtime/deploy/k8s/cronjob-daily.yaml
```

Notes:

- Replace placeholder values in `secret.runtime.example.yaml` before apply.
- `cronjob-daily.yaml` references `protheus-runtime-secrets` as `optional: true` to keep bootstrap fail-closed but migration-friendly.
