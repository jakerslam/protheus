output "release_name" {
  description = "Helm release name"
  value       = helm_release.infring.name
}

output "release_namespace" {
  description = "Helm release namespace"
  value       = helm_release.infring.namespace
}

output "release_status" {
  description = "Helm release status"
  value       = helm_release.infring.status
}
