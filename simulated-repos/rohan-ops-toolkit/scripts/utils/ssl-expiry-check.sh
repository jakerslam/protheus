#!/bin/bash
#
# SSL Certificate Expiration Checker
# Monitors TLS certificate expiry dates for critical endpoints
#
# Usage: ./ssl-expiry-check.sh [domain] [port]
#   domain: Target domain to check (default: api.protheus.io)
#   port:   Port number (default: 443)
#
# Examples:
#   ./ssl-expiry-check.sh
#   ./ssl-expiry-check.sh trading.protheus.io 8443
#
# Exit codes: 0 = valid, 1 = warning (< 30 days), 2 = critical (< 7 days), 3 = error

set -euo pipefail

DOMAIN="${1:-api.protheus.io}"
PORT="${2:-443}"
WARNING_DAYS=30
CRITICAL_DAYS=7

# Colors for terminal output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Get certificate expiry date
get_expiry_date() {
    echo | openssl s_client -servername "$DOMAIN" -connect "$DOMAIN:$PORT" 2>/dev/null | \
        openssl x509 -noout -enddate 2>/dev/null | \
        cut -d= -f2
}

# Calculate days until expiry
calculate_days() {
    local expiry_date="$1"
    local expiry_epoch
    local current_epoch
    local diff_seconds
    local diff_days

    expiry_epoch=$(date -j -f "%b %d %H:%M:%S %Y %Z" "$expiry_date" +%s 2>/dev/null || \
                   date -d "$expiry_date" +%s 2>/dev/null)
    current_epoch=$(date +%s)
    diff_seconds=$(( expiry_epoch - current_epoch ))
    diff_days=$(( diff_seconds / 86400 ))

    echo "$diff_days"
}

main() {
    log_info "Checking SSL certificate for ${DOMAIN}:${PORT}"

    if ! command -v openssl &> /dev/null; then
        log_error "OpenSSL is not installed"
        exit 3
    fi

    expiry_date=$(get_expiry_date)

    if [ -z "$expiry_date" ]; then
        log_error "Could not retrieve certificate for ${DOMAIN}:${PORT}"
        exit 3
    fi

    days_remaining=$(calculate_days "$expiry_date")

    log_info "Certificate expires on: ${expiry_date}"
    log_info "Days remaining: ${days_remaining}"

    if [ "$days_remaining" -lt "$CRITICAL_DAYS" ]; then
        log_error "CRITICAL: Certificate expires in ${days_remaining} days!"
        exit 2
    elif [ "$days_remaining" -lt "$WARNING_DAYS" ]; then
        log_warn "WARNING: Certificate expires in ${days_remaining} days"
        exit 1
    else
        log_info "Certificate is valid for ${days_remaining} days (healthy)"
        exit 0
    fi
}

main "$@"