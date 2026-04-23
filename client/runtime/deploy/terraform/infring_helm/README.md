# Infring Terraform Helm Module

Terraform wrapper for deploying the Infring Helm chart.

## Usage

```hcl
module "infring" {
  source = "./client/runtime/deploy/terraform/infring_helm"

  release_name     = "infring"
  namespace        = "infring"
  image_repository = "protheuslabs/infring"
  image_tag        = "latest"
  existing_secret_name = "infring-runtime-secrets"
  daemon_enabled   = true
  daemon_replicas  = 2
  sso_enabled      = true
  sso_issuer_url   = "https://issuer.example.com"
  sso_client_id    = "infring"
  nvidia_enabled   = false
}
```

## Apply

```bash
terraform init
terraform apply
```
