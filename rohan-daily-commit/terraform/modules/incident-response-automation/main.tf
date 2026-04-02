# Incident Response Automation Module
# Author: Rohan Kapoor (VP Platform & Operations)
# Purpose: Automated incident detection, escalation, and remediation workflows
# Last Updated: 2026-04-01

terraform {
  required_version = ">= 1.5.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    pagerduty = {
      source  = "PagerDuty/pagerduty"
      version = "~> 3.0"
    }
  }
}

#------------------------------------------------------------------------------
# Incident Response EventBridge Rules
#------------------------------------------------------------------------------

resource "aws_cloudwatch_event_rule" "high_severity_alarm" {
  name        = "${var.environment}-high-severity-incident"
  description = "Captures high-severity CloudWatch alarms for immediate escalation"

  event_pattern = jsonencode({
    source      = ["aws.cloudwatch"]
    detail-type = ["CloudWatch Alarm State Change"]
    detail = {
      state = {
        value = ["ALARM"]
      }
      configuration = {
        metrics = {
          metricStat = {
            metric = {
              name = var.critical_metrics
            }
          }
        }
      }
    }
  })
}

resource "aws_cloudwatch_event_rule" "auto_remediation_trigger" {
  name        = "${var.environment}-auto-remediation-trigger"
  description = "Triggers automated remediation for known incident patterns"

  event_pattern = jsonencode({
    source      = ["custom.incident.response"]
    detail-type = ["Incident Detected"]
    detail = {
      severity = ["P2", "P3"]
      auto_remediation_eligible = [true]
    }
  })
}

#------------------------------------------------------------------------------
# Lambda Functions for Incident Response
#------------------------------------------------------------------------------

resource "aws_lambda_function" "incident_orchestrator" {
  function_name = "${var.environment}-incident-orchestrator"
  description   = "Central orchestrator for incident response workflows"
  runtime       = "python3.11"
  handler       = "orchestrator.handler"
  timeout       = 60
  memory_size   = 512

  filename         = data.archive_file.orchestrator_zip.output_path
  source_code_hash = data.archive_file.orchestrator_zip.output_base64sha256

  role = aws_iam_role.lambda_incident_role.arn

  environment {
    variables = {
      PAGERDUTY_SERVICE_KEY   = var.pagerduty_service_key
      SLACK_WEBHOOK_URL       = var.slack_webhook_url
      INCIDENT_TABLE          = aws_dynamodb_table.incidents.name
      AUTO_REMEDIATION_ENABLED = var.auto_remediation_enabled
      ESCALATION_MATRIX_ARN   = aws_secretsmanager_secret.escalation_matrix.arn
    }
  }

  tracing_config {
    mode = "Active"
  }

  tags = merge(var.common_tags, {
    Function = "incident-orchestrator"
  })
}

resource "aws_lambda_function" "auto_remediation" {
  function_name = "${var.environment}-auto-remediation"
  description   = "Executes automated remediation actions for known incident types"
  runtime       = "python3.11"
  handler       = "remediation.handler"
  timeout       = 300
  memory_size   = 1024

  filename         = data.archive_file.remediation_zip.output_path
  source_code_hash = data.archive_file.remediation_zip.output_base64sha256

  role = aws_iam_role.lambda_remediation_role.arn

  environment {
    variables = {
      REMEDIATION_LOG_TABLE = aws_dynamodb_table.remediation_actions.name
      MAX_CONCURRENT_ACTIONS = "5"
      ROLLBACK_ON_FAILURE   = "true"
    }
  }

  tags = merge(var.common_tags, {
    Function = "auto-remediation"
  })
}

#------------------------------------------------------------------------------
# DynamoDB Tables for Incident Tracking
#------------------------------------------------------------------------------

resource "aws_dynamodb_table" "incidents" {
  name         = "${var.environment}-incidents"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "incident_id"
  range_key    = "timestamp"

  attribute {
    name = "incident_id"
    type = "S"
  }

  attribute {
    name = "timestamp"
    type = "N"
  }

  attribute {
    name = "status"
    type = "S"
  }

  attribute {
    name = "severity"
    type = "S"
  }

  global_secondary_index {
    name            = "StatusIndex"
    hash_key        = "status"
    range_key       = "timestamp"
    projection_type = "ALL"
  }

  global_secondary_index {
    name            = "SeverityIndex"
    hash_key        = "severity"
    range_key       = "timestamp"
    projection_type = "ALL"
  }

  point_in_time_recovery {
    enabled = true
  }

  server_side_encryption {
    enabled = true
  }

  tags = var.common_tags
}

resource "aws_dynamodb_table" "remediation_actions" {
  name         = "${var.environment}-remediation-actions"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "action_id"
  range_key    = "executed_at"

  attribute {
    name = "action_id"
    type = "S"
  }

  attribute {
    name = "executed_at"
    type = "N"
  }

  ttl {
    attribute_name = "expires_at"
    enabled        = true
  }

  tags = var.common_tags
}

#------------------------------------------------------------------------------
# Event Targets
#------------------------------------------------------------------------------

resource "aws_cloudwatch_event_target" "orchestrator_target" {
  rule      = aws_cloudwatch_event_rule.high_severity_alarm.name
  target_id = "IncidentOrchestrator"
  arn       = aws_lambda_function.incident_orchestrator.arn
}

resource "aws_cloudwatch_event_target" "remediation_target" {
  rule      = aws_cloudwatch_event_rule.auto_remediation_trigger.name
  target_id = "AutoRemediation"
  arn       = aws_lambda_function.auto_remediation.arn
}

#------------------------------------------------------------------------------
# Lambda Permissions
#------------------------------------------------------------------------------

resource "aws_lambda_permission" "allow_eventbridge_orchestrator" {
  statement_id  = "AllowEventBridgeInvokeOrchestrator"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.incident_orchestrator.function_name
  principal     = "events.amazonaws.com"
  source_arn    = aws_cloudwatch_event_rule.high_severity_alarm.arn
}

resource "aws_lambda_permission" "allow_eventbridge_remediation" {
  statement_id  = "AllowEventBridgeInvokeRemediation"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.auto_remediation.function_name
  principal     = "events.amazonaws.com"
  source_arn    = aws_cloudwatch_event_rule.auto_remediation_trigger.arn
}

#------------------------------------------------------------------------------
# Secrets Manager for Sensitive Config
#------------------------------------------------------------------------------

resource "aws_secretsmanager_secret" "escalation_matrix" {
  name                    = "${var.environment}/incident-response/escalation-matrix"
  description             = "Escalation matrix for incident response"
  recovery_window_in_days = 7

  tags = var.common_tags
}

resource "aws_secretsmanager_secret_version" "escalation_matrix" {
  secret_id = aws_secretsmanager_secret.escalation_matrix.id
  secret_string = jsonencode({
    p1 = {
      initial_response_sla = "5 minutes"
      escalation_time      = "15 minutes"
      contacts             = var.p1_escalation_contacts
    }
    p2 = {
      initial_response_sla = "15 minutes"
      escalation_time      = "45 minutes"
      contacts             = var.p2_escalation_contacts
    }
    p3 = {
      initial_response_sla = "1 hour"
      escalation_time      = "4 hours"
      contacts             = var.p3_escalation_contacts
    }
  })
}

#------------------------------------------------------------------------------
# CloudWatch Alarms for Monitoring the Monitor
#------------------------------------------------------------------------------

resource "aws_cloudwatch_metric_alarm" "incident_lambda_errors" {
  alarm_name          = "${var.environment}-incident-lambda-errors"
  comparison_operator   = "GreaterThanThreshold"
  evaluation_periods    = 2
  metric_name           = "Errors"
  namespace             = "AWS/Lambda"
  period                = 60
  statistic             = "Sum"
  threshold             = 5
  alarm_description   = "Alarm when incident response Lambda errors exceed threshold"
  treat_missing_data    = "notBreaching"

  dimensions = {
    FunctionName = aws_lambda_function.incident_orchestrator.function_name
  }

  tags = var.common_tags
}

resource "aws_cloudwatch_metric_alarm" "incident_table_throttles" {
  alarm_name          = "${var.environment}-incident-table-throttles"
  comparison_operator   = "GreaterThanThreshold"
  evaluation_periods    = 1
  metric_name           = "ThrottledRequests"
  namespace             = "AWS/DynamoDB"
  period                = 60
  statistic             = "Sum"
  threshold             = 0
  alarm_description   = "Detect throttling on incident tracking table"
  treat_missing_data    = "notBreaching"

  dimensions = {
    TableName = aws_dynamodb_table.incidents.name
  }

  tags = var.common_tags
}