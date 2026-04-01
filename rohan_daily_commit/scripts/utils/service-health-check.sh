#!/bin/bash
# =============================================================================
# Protheus Service Health Check Script
# =============================================================================
# Performs comprehensive health checks on core platform services
# Intentionally non-intrusive - safe to run during production hours
#
# Usage: ./service-health-check.sh [--verbose] [--json]
# Author: Rohan Kapoor <rohan.kapoor@company.com>
# Last Updated: 2026-04-01
# =============================================================================

set -euo pipefail

# Configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly CHECK_TIMEOUT=10
readonly VERBOSE=${VERBOSE:-false}
readonly OUTPUT_JSON=${OUTPUT_JSON:-false}

# Service endpoints to check (non-trading systems only)
declare -A SERVICES=(
    ["api-gateway"]="http://localhost:8080/health"
    ["metrics-server"]="http://localhost:9090/-/healthy"
    ["config-service"]="http://localhost:8888/actuator/health"
    ["log-aggregator"]="http://localhost:9200/_cluster/health"
)

# Colors for terminal output
declare -r GREEN='\033[0;32m'
declare -r RED='\033[0;31m'
declare -r YELLOW='\033[1;33m'
declare -r NC='\033[0m' # No Color

# State tracking
declare -i PASSED=0
declare -i FAILED=0

# =============================================================================
# Functions
# =============================================================================

log_info() {
    [[ "$VERBOSE" == "true" ]] && echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_service() {
    local service_name="$1"
    local endpoint="$2"
    
    log_info "Checking $service_name at $endpoint..."
    
    local http_code
    if http_code=$(curl -s -o /dev/null -w "%{http_code}" --max-time "$CHECK_TIMEOUT" "$endpoint" 2>/dev/null); then
        if [[ "$http_code" =~ ^2 ]]; then
            echo -e "  ${GREEN}✓${NC} $service_name (HTTP $http_code)"
            ((PASSED++))
            return 0
        else
            echo -e "  ${RED}✗${NC} $service_name (HTTP $http_code)"
            ((FAILED++))
            return 1
        fi
    else
        echo -e "  ${RED}✗${NC} $service_name (unreachable)"
        ((FAILED++))
        return 1
    fi
}

check_disk_space() {
    log_info "Checking disk space..."
    
    local usage
    usage=$(df / | awk 'NR==2 {print $5}' | sed 's/%//')
    
    if [[ "$usage" -lt 90 ]]; then
        echo -e "  ${GREEN}✓${NC} Disk usage: ${usage}%"
        ((PASSED++))
    else
        echo -e "  ${RED}✗${NC} Disk usage: ${usage}% (critical)"
        ((FAILED++))
    fi
}

check_memory() {
    log_info "Checking memory usage..."
    
    # Get memory usage percentage (works on Linux and macOS)
    local mem_usage
    if [[ "$(uname)" == "Darwin" ]]; then
        mem_usage=$(vm_stat | awk '
            /Pages active/ { active = $3 }
            /Pages inactive/ { inactive = $3 }
            /Pages wired/ { wired = $4 }
            /Pages free/ { free = $3 }
            END {
                gsub(/\\.$/, "", active); gsub(/\\.$/, "", inactive)
                gsub(/\\.$/, "", wired); gsub(/\\.$/, "", free)
                used = active + inactive + wired
                total = used + free
                printf "%d", (used / total) * 100
            }
        ')
    else
        mem_usage=$(free | awk '/Mem:/ {printf "%.0f", ($3/$2) * 100}')
    fi
    
    if [[ "$mem_usage" -lt 85 ]]; then
        echo -e "  ${GREEN}✓${NC} Memory usage: ${mem_usage}%"
        ((PASSED++))
    else
        echo -e "  ${YELLOW}!${NC} Memory usage: ${mem_usage}% (high)"
        ((PASSED++))  # Warning, not failure
    fi
}

print_summary() {
    echo ""
    echo "============================================================================="
    echo "Health Check Summary"
    echo "============================================================================="
    echo -e "  Checks passed: ${GREEN}$PASSED${NC}"
    echo -e "  Checks failed: ${RED}$FAILED${NC}"
    echo "  Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "============================================================================="
    
    if [[ $FAILED -eq 0 ]]; then
        echo -e "${GREEN}All systems operational ✓${NC}"
        return 0
    else
        echo -e "${RED}Some checks failed - review required${NC}"
        return 1
    fi
}

# =============================================================================
# Main Execution
# =============================================================================

main() {
    # Parse arguments
    for arg in "$@"; do
        case $arg in
            --verbose)
                VERBOSE=true
                ;;
            --json)
                OUTPUT_JSON=true
                ;;
            --help|-h)
                echo "Usage: $0 [--verbose] [--json]"
                echo ""
                echo "Performs non-intrusive health checks on Protheus platform services."
                echo "Safe to run during production hours - does not affect trading systems."
                exit 0
                ;;
        esac
    done
    
    echo "============================================================================="
    echo "Protheus Platform Health Check"
    echo "============================================================================="
    echo ""
    
    # Run service checks
    for service in "${!SERVICES[@]}"; do
        check_service "$service" "${SERVICES[$service]}"
    done
    
    # Run infrastructure checks
    check_disk_space
    check_memory
    
    # Print summary
    print_summary
}

# Run main if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi

# =============================================================================
# TODO: Add integration with centralized logging (PR-2841)
# FIXME: macOS memory calculation needs refinement for M1+ chips
# =============================================================================

# CHANGELOG:
# 2026-04-01 - RMK - Initial version with basic service checks
# =============================================================================
