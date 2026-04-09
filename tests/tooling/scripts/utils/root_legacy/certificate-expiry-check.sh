#!/bin/bash
#
# Certificate Expiry Check Utility
# Author: Rohan Kapoor
# Last Updated: 2026-03-25
#
# Purpose: Monitors SSL/TLS certificate expiration dates for all
#          configured endpoints. Provides early warning for upcoming
#          renewals to prevent service disruptions.
#
# Usage: ./certificate-expiry-check.sh [--threshold-days 30] [--notify]
#
# Exit Codes:
#   0 - All certificates valid (not expiring within threshold)
#   1 - One or more certificates expiring soon
#   2 - Check failed (network or config error)
#

set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly CONFIG_FILE="${SCRIPT_DIR}/../../config/endpoints.yaml"
readonly LOG_FILE="${SCRIPT_DIR}/../../logs/cert-check.log"

# Default threshold: warn if expiring within 30 days
THRESHOLD_DAYS=30
NOTIFY=false
VERBOSE=false

# Colors for output (disable if not TTY)
if [[ -t 1 ]]; then
    readonly RED='\033[0;31m'
    readonly YELLOW='\033[1;33m'
    readonly GREEN='\033[0;32m'
    readonly NC='\033[0m'
else
    readonly RED=''
    readonly YELLOW=''
    readonly GREEN=''
    readonly NC=''
fi

log() {
    local msg="[$(date -u +'%Y-%m-%dT%H:%M:%SZ')] $1"
    echo -e "$msg" | tee -a "$LOG_FILE" 2>/dev/null || echo -e "$msg"
}

usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS]

Check SSL/TLS certificate expiration dates for configured endpoints.

OPTIONS:
    -t, --threshold-days DAYS    Warning threshold in days (default: 30)
    -n, --notify                 Send notification if certificates expiring
    -v, --verbose                Enable verbose output
    -h, --help                   Display this help message

EXAMPLES:
    $(basename "$0")                    # Check with default 30-day threshold
    $(basename "$0") -t 14              # Check with 14-day threshold
    $(basename "$0") --notify -v        # Notify and show verbose output

EOF
}

parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -t|--threshold-days)
                THRESHOLD_DAYS="$2"
                shift 2
                ;;
            -n|--notify)
                NOTIFY=true
                shift
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                log "${RED}Error: Unknown option $1${NC}"
                usage
                exit 2
                ;;
        esac
    done
}

check_certificate() {
    local host="$1"
    local port="${2:-443}"
    local threshold_seconds=$((THRESHOLD_DAYS * 24 * 60 * 60))
    
    if ! command -v openssl &> /dev/null; then
        log "${RED}Error: OpenSSL is required but not installed${NC}"
        return 2
    fi
    
    # Get certificate expiry date
    local expiry_date
    if ! expiry_date=$(echo | timeout 10 openssl s_client -connect "${host}:${port}" -servername "$host" 2>/dev/null \
        | openssl x509 -noout -enddate 2>/dev/null \
        | cut -d= -f2); then
        log "${RED}Failed to retrieve certificate for ${host}:${port}${NC}"
        return 2
    fi
    
    # Convert to seconds since epoch
    local expiry_seconds
    expiry_seconds=$(date -j -f "%b %d %T %Y %Z" "$expiry_date" +%s 2>/dev/null || \
                     date -d "$expiry_date" +%s 2>/dev/null)
    
    local current_seconds
    current_seconds=$(date +%s)
    
    local days_until_expiry=$(( (expiry_seconds - current_seconds) / 86400 ))
    
    if [[ $days_until_expiry -lt 0 ]]; then
        log "${RED}CRITICAL: Certificate for ${host} has EXPIRED (${days_until_expiry} days ago)${NC}"
        return 1
    elif [[ $days_until_expiry -lt $THRESHOLD_DAYS ]]; then
        log "${YELLOW}WARNING: Certificate for ${host} expires in ${days_until_expiry} days${NC}"
        return 1
    else
        if [[ "$VERBOSE" == true ]]; then
            log "${GREEN}OK: Certificate for ${host} valid for ${days_until_expiry} days${NC}"
        fi
        return 0
    fi
}

main() {
    parse_args "$@"
    
    log "Starting certificate expiry check (threshold: ${THRESHOLD_DAYS} days)"
    
    local exit_code=0
    local expiring_count=0
    
    # Check endpoints from config if available, otherwise check defaults
    if [[ -f "$CONFIG_FILE" ]]; then
        log "Loading endpoints from ${CONFIG_FILE}"
        # This is a simplified check - in production, parse the YAML properly
        # or use a tool like yq
        while IFS= read -r line; do
            if [[ "$line" =~ host:[[:space:]]*(.+) ]]; then
                host="${BASH_REMATCH[1]}"
                if ! check_certificate "$host"; then
                    ((expiring_count++)) || true
                    exit_code=1
                fi
            fi
        done < "$CONFIG_FILE"
    else
        # Default endpoints to check
        log "Config file not found, checking default endpoints"
        local default_endpoints=("api.protheus.io" "ws.protheus.io" "status.protheus.io")
        for endpoint in "${default_endpoints[@]}"; do
            if ! check_certificate "$endpoint"; then
                ((expiring_count++)) || true
                exit_code=1
            fi
        done
    fi
    
    if [[ $expiring_count -gt 0 ]]; then
        log "${YELLOW}Found ${expiring_count} certificate(s) expiring within ${THRESHOLD_DAYS} days${NC}"
        
        if [[ "$NOTIFY" == true ]]; then
            # Send notification (placeholder - implement based on your alerting system)
            log "Sending notification to on-call team..."
            # Example: curl -X POST "$ALERT_WEBHOOK" -d "${expiring_count} certificates expiring soon"
        fi
    else
        log "${GREEN}All certificates valid (no expiry within ${THRESHOLD_DAYS} days)${NC}"
    fi
    
    log "Certificate check completed with exit code ${exit_code}"
    exit $exit_code
}

main "$@"
