#!/bin/bash
#
# Disk Usage Monitoring Script
# Alerts when disk usage exceeds defined thresholds
# Author: Rohan Kapoor
# Last Modified: 2026-04-04
#

set -euo pipefail

# Configuration
WARNING_THRESHOLD=80
CRITICAL_THRESHOLD=95
CHECK_PATHS=("/var/log" "/tmp" "/opt/protheus/data")
ALERT_WEBHOOK="${ALERT_WEBHOOK_URL:-}"

# Colors for terminal output (when run interactively)
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

log_info() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] INFO: $*"
}

log_warn() {
    echo -e "[$(date +'%Y-%m-%d %H:%M:%S')] ${YELLOW}WARN${NC}: $*"
}

log_error() {
    echo -e "[$(date +'%Y-%m-%d %H:%M:%S')] ${RED}CRIT${NC}: $*"
}

check_disk_usage() {
    local path="$1"
    local usage
    
    if [[ ! -d "$path" ]]; then
        log_warn "Path does not exist: $path"
        return 1
    fi
    
    # Get usage percentage (remove % sign)
    usage=$(df -h "$path" | tail -1 | awk '{print $5}' | tr -d '%')
    
    if [[ "$usage" -ge "$CRITICAL_THRESHOLD" ]]; then
        log_error "CRITICAL: $path is ${usage}% full (threshold: ${CRITICAL_THRESHOLD}%)"
        send_alert "CRITICAL" "$path" "$usage"
        return 2
    elif [[ "$usage" -ge "$WARNING_THRESHOLD" ]]; then
        log_warn "WARNING: $path is ${usage}% full (threshold: ${WARNING_THRESHOLD}%)"
        send_alert "WARNING" "$path" "$usage"
        return 1
    else
        log_info "OK: $path is ${usage}% full"
        return 0
    fi
}

send_alert() {
    local level="$1"
    local path="$2"
    local usage="$3"
    
    if [[ -n "$ALERT_WEBHOOK" ]]; then
        # JSON payload for webhook
        local payload
        payload=$(cat <<EOF
{
    "level": "$level",
    "service": "disk-monitor",
    "message": "Disk usage $level on $(hostname)",
    "path": "$path",
    "usage_percent": $usage,
    "threshold": ${level//WARNING/$WARNING_THRESHOLD},
    "timestamp": "$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
}
EOF
)
        # Send alert (best effort - don't fail if webhook unavailable)
        curl -s -X POST -H "Content-Type: application/json" \
            -d "$payload" "$ALERT_WEBHOOK" || true
    fi
}

main() {
    log_info "Starting disk usage check..."
    
    local exit_code=0
    local critical_found=0
    
    for path in "${CHECK_PATHS[@]}"; do
        if ! check_disk_usage "$path"; then
            local rc=$?
            if [[ $rc -eq 2 ]]; then
                critical_found=1
            fi
            exit_code=1
        fi
    done
    
    if [[ $critical_found -eq 1 ]]; then
        log_error "One or more paths are at CRITICAL disk usage levels"
        exit 2
    elif [[ $exit_code -ne 0 ]]; then
        log_warn "One or more paths exceeded warning thresholds"
        exit 1
    fi
    
    log_info "Disk usage check completed successfully"
}

# Run main function
main "$@"
