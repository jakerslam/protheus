#------------------------------------------------------------------------------
# Incident Response Automation - Variables
#------------------------------------------------------------------------------

variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string

  validation {
    condition     = contains(["dev", "staging", "prod"], var.environment)
    error_message = "Environment must be dev, staging, or prod."
  }
}

variable "region" {
  description = "AWS region for deployment"
  type        = string
  default     = "us-west-2"
}

variable "critical_metrics" {
  description = "List of CloudWatch metric names that trigger P1 incidents"
  type        = list(string)
  default     = ["ErrorRate", "Latency", "DatabaseConnections", "CPUUtilization"]
}

variable "auto_remediation_enabled" {
  description = "Enable automatic remediation for eligible incidents"
  type        = bool
  default     = true
}

variable "pagerduty_service_key" {
  description = "PagerDuty service integration key"
  type        = string
  sensitive   = true
}

variable "slack_webhook_url" {
  description = "Slack webhook URL for incident notifications"
  type        = string
  sensitive   = true
}

variable "p1_escalation_contacts" {
  description = "List of P1 escalation contacts"
  type        = list(string)
}

variable "p2_escalation_contacts" {
  description = "List of P2 escalation contacts"
  type        = list(string)
}

variable "p3_escalation_contacts" {
  description = "List of P3 escalation contacts"
  type        = list(string)
}

variable "retention_days" {
  description = "Number of days to retain incident data"
  type        = number
  default     = 90
}

variable "common_tags" {
  description = "Common tags applied to all resources"
  type        = map(string)
  default = {
    ManagedBy = "terraform"
    Owner     = "platform-operations"
    Purpose   = "incident-response"
  }
}

variable "runbook_s3_bucket" {
  description = "S3 bucket containing runbook documentation"
  type        = string
}

variable "incident_channel_mapping" {
  description = "Mapping of severity to Slack channels"
  type        = map(string)
  default = {
    P1 = "#incidents-critical"
    P2 = "#incidents-high"
    P3 = "#incidents-standard"
  }
}

variable "compliance_mode" {
  description = "Enable compliance audit logging (SOC2/ISO27001)"
  type        = bool
  default     = true
}