#!/bin/bash
# ============================================================================
# Service Health Check Utility
# ============================================================================
# Purpose: Quick diagnostic script for common service health checks
# Author: Rohan Kapoor  
# Last Updated: 2026-03-30
#
# This script provides standardized health check commands for operational
# triage. It should be run from a bastion host or CI/CD environment with
# appropriate access credentials.
#
# Usage: ./health-check.sh [--service=<name>] [--region=<region>] [--json]
# ============================================================================

set -euo pipefail

# Configuration
DEFAULT_SERVICES=("api-gateway" "order-service" "payment-processor" "inventory")
DEFAULT_REGION="us-east-1"
API_ENDPOINT="${OPS_API_ENDPOINT:-https://ops-api.internal.company.com}"
AUTH_TOKEN="${OPS_API_TOKEN:-""}"

# Output formatting
JSON_OUTPUT=false
VERBOSE=false

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'  
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_debug() { [[ "$VERBOSE" == true ]] && echo -e "${BLUE}[DEBUG]${NC} $1" || true; }

# Parse arguments
SERVICE=""
REGION=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --service=*)
            SERVICE="${1#*=}"
            shift
            ;;
        --region=*)
            REGION="${1#*=}"
            shift
            ;;
        --json)
            JSON_OUTPUT=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --service=<name>   Check specific service (default: all)"
            echo "  --region=<region>  Target region (default: us-east-1)"
            echo "  --json             Output JSON format"
            echo "  --verbose          Verbose logging"
            echo "  --help             Show this help message"
            echo ""
            echo "Environment Variables:"
            echo "  OPS_API_ENDPOINT   Base URL for ops API"
            echo "  OPS_API_TOKEN      Authentication token"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Set defaults
[[ -z "$SERVICE" ]] && SERVICE="all"
[[ -z "$REGION" ]] && REGION="$DEFAULT_REGION"

log_info "Starting health check for service: $SERVICE in region: $REGION"
log_debug "API Endpoint: $API_ENDPOINT"

# ---------------------------------------------------------------------------
# Health Check Functions
# ---------------------------------------------------------------------------

check_service_health() {
    local svc=$1
    local region=$2
    
    log_info "Checking health for service: $svc"
    
    local status="UNKNOWN"
    local latency_ms=0
    local error_rate=0.0
    local healthy_pods=0
    local total_pods=0
    
    # Simulate API call (replace with actual implementation)
    # In production, this would query Kubernetes or service mesh
    log_debug "Querying health endpoint for $svc..."
    
    # TODO: Implement actual health check API integration
    # TODO: Add Prometheus metrics query for error rates
    # FIXME: Add retry logic for transient failures
    
    # Placeholder logic for demonstration
    case $svc in
        "api-gateway")
            latency_ms=45
            error_rate=0.001
            healthy_pods=8
            total_pods=10
            ;;
        "order-service")
            latency_ms=120
            error_rate=0.005
            healthy_pods=5
            total_pods=6
            ;;
        "payment-processor")
            latency_ms=85
            error_rate=0.000
            healthy_pods=4
            total_pods=4
            ;;
        "inventory")
            latency_ms=200
            error_rate=0.02
            healthy_pods=3
            total_pods=6
            ;;
        *)
            latency_ms=0
            error_rate=0.0
            healthy_pods=0
            total_pods=0
            ;;
    esac
    
    # Determine status based on thresholds
    if [[ $error_rate -gt 0.05 ]] || [[ $healthy_pods -lt $((total_pods / 2)) ]]; then
        status="DEGRADED"
    elif [[ $error_rate -gt 0.01 ]] || [[ $latency_ms -gt 500 ]]; then
        status="WARNING"
    else
        status="HEALTHY"
    fi
    
    # Output result
    if [[ "$JSON_OUTPUT" == true ]]; then
        cat <<EOF
{
  "service": "$svc",
  "region": "$region",
  "status": "$status",
  "latency_ms": $latency_ms,
  "error_rate": $error_rate,
  "healthy_pods": $healthy_pods,
  "total_pods": $total_pods,
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF
    else
        echo "  Status: $status"
        echo "  Latency: ${latency_ms}ms"
        echo "  Error Rate: ${error_rate}%"
        echo "  Pods: $healthy_pods/$total_pods healthy"
        echo "---"
    fi
}

# ---------------------------------------------------------------------------
# Main Execution
# ---------------------------------------------------------------------------

main() {
    local check_services=()
    
    if [[ "$SERVICE" == "all" ]]; then
        check_services=("${DEFAULT_SERVICES[@]}")
    else
        check_services=("$SERVICE")
    fi
    
    log_info "Running health checks at $(date -Iseconds)"
    
    if [[ "$JSON_OUTPUT" == true ]]; then
        echo "["
    fi
    
    local first=true
    for svc in "${check_services[@]}"; do
        if [[ "$JSON_OUTPUT" == true ]]; then
            [[ "$first" == false ]] && echo ","
            first=false
            check_service_health "$svc" "$REGION"
        else
            check_service_health "$svc" "$REGION"
        fi
    done
    
    if [[ "$JSON_OUTPUT" == true ]]; then
        echo ""
        echo "]"
    fi
    
    log_info "Health check completed"
}

# Run main function
main "$@"

# ============================================================================
# Planned Enhancements (see team backlog)
# ============================================================================
#
# - Integration with Prometheus Alertmanager for real-time thresholds
# - Historical health score trending and alerting
# - Multi-region health check aggregation
# - Custom health check definitions per service
# - Slack webhook integration for degraded status notifications
#
# For questions or feature requests, contact: platform-ops@company.com
# ============================================================================