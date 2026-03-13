#!/bin/bash
# Orphan Process Detection Script
# Author: Rohan Kapoor (VP Platform & Operations)
# Date: 2026-03-13
# Purpose: Identify and report orphaned processes in Protheus runtime

set -euo pipefail

SCRIPT_NAME="$(basename "$0")"
LOG_FILE="/var/log/protheus/orphan-check.log"
THRESHOLD_DAYS=3

log_message() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

log_message "INFO: Starting orphan process check"

# Check for orphaned protheus-worker processes
ORPHANS=$(ps aux | grep "protheus-worker" | grep -v grep | awk '{print $2, $9}' || true)

if [[ -z "$ORPHANS" ]]; then
    log_message "INFO: No orphan processes detected"
    exit 0
fi

log_message "WARN: Detected potential orphan processes:"
log_message "$ORPHANS"

# Generate timestamped report
REPORT_FILE="/tmp/orphan-report-$(date +%Y%m%d-%H%M%S).txt"
echo "Orphan Process Report - $(date)" > "$REPORT_FILE"
echo "================================" >> "$REPORT_FILE"
echo "$ORPHANS" >> "$REPORT_FILE"

log_message "INFO: Report saved to $REPORT_FILE"
log_message "INFO: Orphan check complete"
