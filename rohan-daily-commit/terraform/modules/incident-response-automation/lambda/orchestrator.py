"""
Incident Response Orchestrator Lambda

Central orchestration hub for incident response workflows.
Handles detection, classification, escalation, and notification.

Author: Rohan Kapoor
Last Updated: 2026-04-01
"""

import json
import os
import boto3
import hashlib
from datetime import datetime, timezone
from typing import Dict, Any, Optional
import urllib.request
import urllib.error

# AWS Clients
dynamodb = boto3.resource('dynamodb')
secretsmanager = boto3.client('secretsmanager')
sns = boto3.client('sns')

table = dynamodb.Table(os.environ.get('INCIDENT_TABLE', 'incidents'))

SEVERITY_WEIGHTS = {'P1': 1, 'P2': 2, 'P3': 3, 'P4': 4}


def handler(event: Dict[str, Any], context: Any) -> Dict[str, Any]:
    """
    Main Lambda handler for incident events.
    
    Args:
        event: CloudWatch/EventBridge event containing alarm details
        context: Lambda context
        
    Returns:
        Response dict with incident_id and actions taken
    """
    print(f"Received event: {json.dumps(event)}")
    
    try:
        # Parse and validate the event
        incident_data = parse_event(event)
        
        # Classify incident severity
        severity = classify_severity(incident_data)
        incident_data['severity'] = severity
        
        # Generate unique incident ID
        incident_id = generate_incident_id(incident_data)
        incident_data['incident_id'] = incident_id
        
        # Record incident in DynamoDB
        record_incident(incident_data)
        
        # Execute workflow based on severity
        actions_taken = execute_response_workflow(incident_data)
        
        # Check if auto-remediation is applicable
        if should_auto_remediate(incident_data):
            trigger_auto_remediation(incident_id, incident_data)
            actions_taken.append('auto_remediation_triggered')
        
        return {
            'statusCode': 200,
            'incident_id': incident_id,
            'severity': severity,
            'actions_taken': actions_taken
        }
        
    except Exception as e:
        print(f"Error processing incident: {str(e)}")
        # Attempt emergency notification on critical errors
        send_emergency_alert(f"Incident orchestrator failed: {str(e)}")
        raise


def parse_event(event: Dict[str, Any]) -> Dict[str, Any]:
    """Extract relevant data from CloudWatch/EventBridge event."""
    detail = event.get('detail', {})
    
    return {
        'timestamp': int(datetime.now(timezone.utc).timestamp()),
        'source': event.get('source', 'unknown'),
        'alarm_name': detail.get('alarmName', 'Unknown Alarm'),
        'alarm_description': detail.get('configuration', {}).get('description', ''),
        'state': detail.get('state', {}).get('value', 'UNKNOWN'),
        'region': event.get('region', 'us-west-2'),
        'account': event.get('account', 'unknown'),
        'raw_event': event,
        'status': 'OPEN'
    }


def classify_severity(incident_data: Dict[str, Any]) -> str:
    """Classify incident severity based on alarm metadata."""
    alarm_name = incident_data.get('alarm_name', '').lower()
    
    # P1 Critical: Customer-impacting, revenue-impacting, security
    p1_keywords = ['production', 'critical', 'security', 'breach', 'outage', '0 errors']
    if any(kw in alarm_name for kw in p1_keywords):
        return 'P1'
    
    # P2 High: Degraded service, performance issues
    p2_keywords = ['error rate', 'latency', 'throttle', 'capacity', 'degraded']
    if any(kw in alarm_name for kw in p2_keywords):
        return 'P2'
    
    # P3 Medium: Non-urgent issues
    return 'P3'


def generate_incident_id(incident_data: Dict[str, Any]) -> str:
    """Generate deterministic but unique incident ID."""
    base = f"{incident_data['alarm_name']}:{incident_data['timestamp']}"
    hash_digest = hashlib.sha256(base.encode()).hexdigest()[:12]
    return f"INC-{datetime.now(timezone.utc).strftime('%Y%m%d')}-{hash_digest.upper()}"


def record_incident(incident_data: Dict[str, Any]) -> None:
    """Record incident details to DynamoDB."""
    item = {
        'incident_id': incident_data['incident_id'],
        'timestamp': incident_data['timestamp'],
        'severity': incident_data['severity'],
        'status': incident_data['status'],
        'source': incident_data['source'],
        'alarm_name': incident_data['alarm_name'],
        'region': incident_data['region'],
        'account': incident_data['account'],
        'ttl': incident_data['timestamp'] + (90 * 24 * 60 * 60)  # 90 days
    }
    
    # Store full event in S3 for large payloads (compliance requirement)
    if os.environ.get('COMPLIANCE_MODE', 'true').lower() == 'true':
        s3 = boto3.client('s3')
        audit_bucket = os.environ.get('AUDIT_BUCKET')
        if audit_bucket:
            audit_key = f"incidents/{item['incident_id']}/event.json"
            s3.put_object(
                Bucket=audit_bucket,
                Key=audit_key,
                Body=json.dumps(incident_data['raw_event']),
                ServerSideEncryption='AES256'
            )
            item['audit_s3_key'] = audit_key
    
    table.put_item(Item=item)
    print(f"Recorded incident {item['incident_id']} to DynamoDB")


def execute_response_workflow(incident_data: Dict[str, Any]) -> list:
    """Execute appropriate response workflow based on severity."""
    actions = []
    severity = incident_data['severity']
    
    # All severities: Log and notify
    actions.append('incident_logged')
    
    # P1/P2: Immediate Slack notification
    if severity in ['P1', 'P2']:
        send_slack_notification(incident_data)
        actions.append('slack_notified')
    
    # P1: PagerDuty escalation
    if severity == 'P1':
        create_pagerduty_incident(incident_data)
        actions.append('pagerduty_escalated')
        
        # Also send SMS for critical alerts
        send_sms_notification(incident_data)
        actions.append('sms_sent')
    
    # All: Update runbook tracking
    tag_with_runbook(incident_data)
    actions.append('runbook_tagged')
    
    return actions


def should_auto_remediate(incident_data: Dict[str, Any]) -> bool:
    """Determine if incident qualifies for auto-remediation."""
    if os.environ.get('AUTO_REMEDIATION_ENABLED', 'true').lower() != 'true':
        return False
    
    # Only P2/P3 eligible for auto-remediation
    if incident_data['severity'] not in ['P2', 'P3']:
        return False
    
    # Check if alarm type is in allowlist
    auto_remediate_alarms = [
        'disk-full',
        'memory-pressure',
        'stuck-queue',
        'stale-connection'
    ]
    
    alarm_name = incident_data.get('alarm_name', '').lower()
    return any(pattern in alarm_name for pattern in auto_remediate_alarms)


def trigger_auto_remediation(incident_id: str, incident_data: Dict[str, Any]) -> None:
    """Trigger the auto-remediation Lambda."""
    lambda_client = boto3.client('lambda')
    
    remediation_event = {
        'incident_id': incident_id,
        'alarm_name': incident_data['alarm_name'],
        'severity': incident_data['severity'],
        'region': incident_data['region'],
        'triggered_at': datetime.now(timezone.utc).isoformat()
    }
    
    lambda_client.invoke(
        FunctionName=os.environ.get('AUTO_REMEDIATION_FUNCTION', 'auto-remediation'),
        InvocationType='Event',  # Async
        Payload=json.dumps(remediation_event)
    )
    
    print(f"Triggered auto-remediation for incident {incident_id}")


def send_slack_notification(incident_data: Dict[str, Any]) -> None:
    """Send formatted Slack notification."""
    webhook_url = os.environ.get('SLACK_WEBHOOK_URL')
    if not webhook_url:
        print("Slack webhook not configured")
        return
    
    severity_emoji = {'P1': '🔴', 'P2': '🟠', 'P3': '🟡', 'P4': '🟢'}
    channel_mapping = json.loads(os.environ.get('INCIDENT_CHANNEL_MAPPING', '{}'))
    channel = channel_mapping.get(incident_data['severity'], '#incidents')
    
    message = {
        "channel": channel,
        "username": "IncidentBot",
        "icon_emoji": ":fire:",
        "attachments": [{
            "color": "danger" if incident_data['severity'] == 'P1' else "warning",
            "title": f"{severity_emoji.get(incident_data['severity'], '⚪')} {incident_data['severity']} Incident Detected",
            "fields": [
                {"title": "Incident ID", "value": incident_data['incident_id'], "short": True},
                {"title": "Alarm", "value": incident_data['alarm_name'], "short": True},
                {"title": "Region", "value": incident_data['region'], "short": True},
                {"title": "Time", "value": datetime.fromtimestamp(incident_data['timestamp'], tz=timezone.utc).isoformat(), "short": True}
            ],
            "footer": "Incident Response System",
            "ts": incident_data['timestamp']
        }]
    }
    
    try:
        req = urllib.request.Request(
            webhook_url,
            data=json.dumps(message).encode(),
            headers={'Content-Type': 'application/json'}
        )
        urllib.request.urlopen(req)
    except Exception as e:
        print(f"Failed to send Slack notification: {e}")


def create_pagerduty_incident(incident_data: Dict[str, Any]) -> None:
    """Create PagerDuty incident via Events API v2."""
    # Implementation would use PagerDuty Events API
    # Using events API to avoid PD client dependency
    print(f"PagerDuty incident creation simulated for {incident_data['incident_id']}")


def send_sms_notification(incident_data: Dict[str, Any]) -> None:
    """Send SMS via SNS for P1 incidents."""
    message = f"P1 Incident: {incident_data['alarm_name']} - {incident_data['incident_id']}"
    
    # Get on-call numbers from escalation matrix
    try:
        response = secretsmanager.get_secret_value(
            SecretId=os.environ.get('ESCALATION_MATRIX_ARN')
        )
        matrix = json.loads(response['SecretString'])
        
        for contact in matrix.get('p1', {}).get('contacts', []):
            if contact.startswith('+'):  # Phone number format
                sns.publish(
                    Message=message,
                    PhoneNumber=contact
                )
    except Exception as e:
        print(f"SMS notification failed: {e}")


def tag_with_runbook(incident_data: Dict[str, Any]) -> None:
    """Associate relevant runbook with incident."""
    # Runbook lookup based on alarm patterns
    runbook_mapping = {
        'cpu': 'runbooks/performance/cpu-throttling.md',
        'memory': 'runbooks/performance/memory-pressure.md',
        'disk': 'runbooks/infrastructure/disk-full.md',
        'error': 'runbooks/application/error-rate-spike.md',
        'latency': 'runbooks/application/latency-investigation.md',
        'security': 'runbooks/security/security-event-response.md'
    }
    
    alarm_name = incident_data.get('alarm_name', '').lower()
    runbook = 'runbooks/general/generic-incident-response.md'
    
    for keyword, path in runbook_mapping.items():
        if keyword in alarm_name:
            runbook = path
            break
    
    # Update incident with runbook reference
    table.update_item(
        Key={
            'incident_id': incident_data['incident_id'],
            'timestamp': incident_data['timestamp']
        },
        UpdateExpression='SET runbook_path = :path',
        ExpressionAttributeValues={':path': runbook}
    )


def send_emergency_alert(message: str) -> None:
    """Last-resort alert when orchestrator itself fails."""
    try:
        sns.publish(
            TopicArn=os.environ.get('EMERGENCY_TOPIC_ARN'),
            Subject='CRITICAL: Incident Orchestrator Failure',
            Message=message
        )
    except Exception:
        print(f"Failed to send emergency alert: {message}")