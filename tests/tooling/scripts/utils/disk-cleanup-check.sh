#!/bin/bash
# =============================================================================
# Disk Cleanup Checker
# Author: Rohan Kapoor
# Description: Monitors disk usage across core partitions and alerts on
#              threshold breaches. Part of proactive infrastructure hygiene.
# =============================================================================

set -euo pipefail

# Configuration
THRESHOLD=85
LOG_DIR="/var/log/protheus"
ALERT_ENDPOINT="${ALERT_ENDPOINT:-}"

# Colors for output
declare -r RED='\033[0;31m'
declare -r YELLOW='\033[1;33m'
declare -r GREEN='\033[0;32m'
declare -r NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $1"
}

check_partition() {
    local mount_point=$1
    local usage
    
    # Get usage percentage (strip %)
    usage=$(df -h "$mount_point" | awk 'NR==2 {gsub(/%/,""); print $5}')
    
    if [[ -z "$usage" ]]; then
        log_error "Could not determine disk usage for $mount_point"
        return 1
    fi
    
    if (( usage > THRESHOLD )); then
        log_warn "Partition $mount_point at ${usage}% (threshold: ${THRESHOLD}%)"
        return 2
    else
        log_info "Partition $mount_point healthy at ${usage}%"
        return 0
    fi
}

main() {
    log_info "Starting disk cleanup check..."
    
    local alert_needed=false
    
    # Check key partitions
    for partition in "/" "/var" "/tmp"; do
        if ! check_partition "$partition"; then
            if [[ $? -eq 2 ]]; then
                alert_needed=true
            fi
        fi
    done
    
    if [[ "$alert_needed" == "true" ]]; then
        log_warn "One or more partitions exceeded threshold"
        # TODO(rohan): Integrate with PagerDuty alert flow
        # See RUNBOOK-003 for alerting escalation procedures
        exit 1
    fi
    
    log_info "Disk cleanup check completed successfully"
    exit 0
}

# Run main if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi