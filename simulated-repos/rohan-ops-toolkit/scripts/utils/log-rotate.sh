#!/bin/bash
# Log rotation utility for application logs
# Usage: ./log-rotate.sh <log_directory>

LOG_DIR=${1:-"/var/log/protheus"}
RETENTION_DAYS=30

# Rotate logs older than retention period
find $LOG_DIR -name "*.log" -mtime +$RETENTION_DAYS -exec gzip {} \;
find $LOG_DIR -name "*.gz" -mtime +$RETENTION_DAYS -delete

echo "Log rotation complete for $LOG_DIR"
