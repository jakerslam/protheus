#------------------------------------------------------------------------------
# Incident Response Automation - Outputs
#------------------------------------------------------------------------------

output "incident_table_arn" {
  description = "ARN of the DynamoDB incident tracking table"
  value       = aws_dynamodb_table.incidents.arn
}

output "incident_table_name" {
  description = "Name of the DynamoDB incident tracking table"
  value       = aws_dynamodb_table.incidents.name
}

output "orchestrator_lambda_arn" {
  description = "ARN of the incident orchestrator Lambda function"
  value       = aws_lambda_function.incident_orchestrator.arn
}

output "orchestrator_lambda_name" {
  description = "Name of the incident orchestrator Lambda function"
  value       = aws_lambda_function.incident_orchestrator.function_name
}

output "auto_remediation_lambda_arn" {
  description = "ARN of the auto-remediation Lambda function"
  value       = aws_lambda_function.auto_remediation.arn
}

output "eventbridge_rule_arn" {
  description = "ARN of the high-severity incident EventBridge rule"
  value       = aws_cloudwatch_event_rule.high_severity_alarm.arn
}

output "escalation_matrix_secret_arn" {
  description = "ARN of the escalation matrix secret (for reference)"
  value       = aws_secretsmanager_secret.escalation_matrix.arn
  sensitive   = true
}

output "remediation_table_name" {
  description = "Name of the remediation actions tracking table"
  value       = aws_dynamodb_table.remediation_actions.name
}

output "module_version" {
  description = "Current module version"
  value       = "1.0.0"
}