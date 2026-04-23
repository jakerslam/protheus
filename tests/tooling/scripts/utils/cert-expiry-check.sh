#!/usr/bin/env bash
#
# Utility: Certificate Expiry Monitor
# Author: Rohan Kapoor
# Created: 2026-03-12
#
# Checks SSL/TLS certificate expiration dates for configured endpoints.
# Alerts when certificates are approaching expiration (default: 30 days).
#
# Usage: ./scripts/utils/cert-expiry-check.sh [--days N] [--endpoint host:port]
#
# Exit codes:
#   0 - All certificates valid
#   1 - One or more certificates expiring soon
#   2 - Configuration or connection error
#

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
DEFAULT_WARNING_DAYS=30
VERBOSE=0

# Default endpoints to check (can be overridden via config file)
ENDPOINTS=(
  "api.infring.io:443"
  "grafana.infring.io:443"
  "logs.infring.io:443"
)

# Load custom endpoints from config if available
CONFIG_FILE="${WORKSPACE_ROOT}/.cert-check-config"
if [[ -f "$CONFIG_FILE" ]]; then
  # shellcheck source=/dev/null
  source "$CONFIG_FILE"
fi

# Parse arguments
WARNING_DAYS=$DEFAULT_WARNING_DAYS
CHECK_ENDPOINTS=()

while [[ $# -gt 0 ]]; do
  case $1 in
    --days)
      WARNING_DAYS="$2"
      shift 2
      ;;
    --endpoint)
      CHECK_ENDPOINTS+=("$2")
      shift 2
      ;;
    --verbose)
      VERBOSE=1
      shift
      ;;
    --help|-h)
      echo "Usage: $0 [--days N] [--endpoint host:port] [--verbose]"
      echo "  --days N       Warning threshold in days (default: $DEFAULT_WARNING_DAYS)"
      echo "  --endpoint     Check specific endpoint (can be used multiple times)"
      echo "  --verbose      Enable detailed output"
      echo "  --help         Show this help message"
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 2
      ;;
  esac
done

# Use command-line endpoints if provided, otherwise use defaults
if [[ ${#CHECK_ENDPOINTS[@]} -gt 0 ]]; then
  ENDPOINTS=("${CHECK_ENDPOINTS[@]}")
fi

# Colors for terminal output
if [[ -t 1 ]]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  BLUE='\033[0;34m'
  NC='\033[0m'
else
  RED=''
  GREEN=''
  YELLOW=''
  BLUE=''
  NC=''
fi

# Logging functions
log_info() {
  echo -e "${BLUE}[INFO]${NC} $1"
}

log_pass() {
  echo -e "${GREEN}[PASS]${NC} $1"
}

log_warn() {
  echo -e "${YELLOW}[WARN]${NC} $1"
}

log_fail() {
  echo -e "${RED}[FAIL]${NC} $1"
}

# Check certificate for a single endpoint
check_certificate() {
  local endpoint="$1"
  local host port
  
  # Parse host:port
  if [[ "$endpoint" =~ ^([^:]+):([0-9]+)$ ]]; then
    host="${BASH_REMATCH[1]}"
    port="${BASH_REMATCH[2]}"
  else
    host="$endpoint"
    port=443
  fi
  
  [[ $VERBOSE -eq 1 ]] && log_info "Checking $host:$port..."
  
  # Get certificate expiry date using openssl
  local expiry_date expiry_epoch days_until
  
  expiry_date=$(echo | openssl s_client -servername "$host" -connect "$host:$port" 2>/dev/null | \
    openssl x509 -noout -enddate 2>/dev/null | cut -d= -f2) || {
    log_fail "Could not retrieve certificate for $host:$port"
    return 1
  }
  
  # Convert expiry date to epoch
  expiry_epoch=$(date -j -f "%b %d %H:%M:%S %Y %Z" "$expiry_date" +%s 2>/dev/null || \
                 date -d "$expiry_date" +%s 2>/dev/null) || {
    log_fail "Could not parse expiry date: $expiry_date"
    return 1
  }
  
  local current_epoch
  current_epoch=$(date +%s)
  
  # Calculate days until expiry
  days_until=$(( (expiry_epoch - current_epoch) / 86400 ))
  
  if [[ $days_until -lt 0 ]]; then
    log_fail "$host:$port - EXPIRED ($days_until days ago)"
    return 1
  elif [[ $days_until -le $WARNING_DAYS ]]; then
    log_warn "$host:$port - Expires in $days_until days ($expiry_date)"
    return 1
  else
    log_pass "$host:$port - Valid for $days_until days"
    return 0
  fi
}

# Main execution
main() {
  echo "=== Certificate Expiry Monitor ==="
  echo "Timestamp: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  echo "Warning threshold: $WARNING_DAYS days"
  echo "Endpoints to check: ${#ENDPOINTS[@]}"
  echo ""
  
  local exit_code=0
  
  for endpoint in "${ENDPOINTS[@]}"; do
    if ! check_certificate "$endpoint"; then
      exit_code=1
    fi
  done
  
  echo ""
  if [[ $exit_code -eq 0 ]]; then
    log_pass "All certificates are valid beyond the warning threshold"
  else
    log_warn "One or more certificates require attention"
  fi
  
  exit $exit_code
}

main "$@"
