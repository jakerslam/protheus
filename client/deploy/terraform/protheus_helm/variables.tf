variable "release_name" {
  description = "Helm release name"
  type        = string
  default     = "protheus"
}

variable "namespace" {
  description = "Kubernetes namespace for Protheus"
  type        = string
  default     = "protheus"
}

variable "chart_path" {
  description = "Path to the Protheus Helm chart"
  type        = string
  default     = "../../helm/protheus"
}

variable "image_repository" {
  description = "Container image repository"
  type        = string
  default     = "protheuslabs/protheus"
}

variable "image_tag" {
  description = "Container image tag"
  type        = string
  default     = "latest"
}

variable "cron_schedule" {
  description = "Cron schedule for spine workload"
  type        = string
  default     = "0 */4 * * *"
}

variable "existing_secret_name" {
  description = "Existing Kubernetes secret name for runtime credentials (optional)"
  type        = string
  default     = ""
}

variable "secret_optional" {
  description = "Whether the runtime secret reference is optional"
  type        = bool
  default     = true
}
