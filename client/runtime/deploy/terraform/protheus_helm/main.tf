terraform {
  required_version = ">= 1.5.0"

  required_providers {
    helm = {
      source  = "hashicorp/helm"
      version = ">= 2.11.0"
    }
  }
}

resource "helm_release" "protheus" {
  name             = var.release_name
  namespace        = var.namespace
  create_namespace = true

  chart = var.chart_path

  values = [
    yamlencode({
      image = {
        repository = var.image_repository
        tag        = var.image_tag
      }
      cron = {
        schedule = var.cron_schedule
      }
      daemon = {
        enabled  = var.daemon_enabled
        replicas = var.daemon_replicas
      }
      sso = {
        enabled   = var.sso_enabled
        issuerUrl = var.sso_issuer_url
        clientId  = var.sso_client_id
      }
      nvidia = {
        enabled          = var.nvidia_enabled
        runtimeClassName = var.nvidia_runtime_class_name
      }
      secrets = {
        existingSecretName = var.existing_secret_name
        optional           = var.secret_optional
      }
    })
  ]
}
