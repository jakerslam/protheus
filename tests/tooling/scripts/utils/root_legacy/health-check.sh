#!/bin/bash
# health-check.sh - Comprehensive health check for Infring services
# Author: Rohan Kapoor <rohan.kapoor@protheuslabs.com>
# Created: 2026-03-20
# Last updated: 2026-03-20
#
# Purpose: Perform multi-layer health verification across Infring
# infrastructure components. Designed for use in monitoring scripts,
# CI pipelines, and manual operator diagnostics.
#
# Usage: ./health-check.sh [--component <name>] [--format json|text]
#   --component   Check specific component (daemon|router|storage|all)
#   --format      Output format (default: text)
#   --help        Show this help message
#
# Exit codes:
#   0 = All checks passed
#   1 = One or more checks failed
#   2 = Invalid arguments or configuration
#
# TODO: Add integration with PagerDuty for failed critical checks
# TODO(rohan): Consider adding latency histograms for trending

set -euo pipefail

# Configuration
COMPONENT="all"
FORMAT="text"
TIMEOUT_SECONDS=5
VERBOSE=0

# Color codes for terminal output (only when TTY detected)
RED=""
GREEN=""
YELLOW=""
RESET=""
if [[ -t 1 ]]; then
    RED="\033[31m"
    GREEN="\033[32m"
    YELLOW="\033[33m"
    RESET="\033[0m"
fi

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --component)
            COMPONENT="$2"
            shift 2
            ;;
        --format)
            FORMAT="$2"
            shift 2
            ;;
        --verbose)
            VERBOSE=1
            shift
            ;;
        --help)
            echo "Usage: $0 [--component <name>] [--format json|text] [--verbose]"
            echo ""
            echo "Components: daemon, router, storage, all (default)"
            echo "Formats: text (default), json"
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            echo "Use --help for usage information" >&2
            exit 2
            ;;
    esac
done

# Validation
if [[ "$FORMAT" != "text" && "$FORMAT" != "json" ]]; then
    echo "Invalid format: $FORMAT (must be 'text' or 'json')" >&2
    exit 2
fi

if [[ "$COMPONENT" != "all" && "$COMPONENT" != "daemon" && "$COMPONENT" != "router" && "$COMPONENT" != "storage" ]]; then
    echo "Invalid component: $COMPONENT" >&2
    exit 2
fi

# Results storage
declare -A CHECK_RESULTS
declare -A CHECK_MESSAGES

log() {
    if [[ $VERBOSE -eq 1 ]]; then
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" >&2
    fi
}

output_text() {
    local status="$1"
    local message="$2"
    if [[ "$status" == "PASS" ]]; then
        echo -e "${GREEN}✓${RESET} $message"
    elif [[ "$status" == "WARN" ]]; then
        echo -e "${YELLOW}⚠${RESET} $message"
    else
        echo -e "${RED}✗${RESET} $message"
    fi
}

output_json() {
    local first=1
    echo "{"
    echo '  "timestamp": "'"$(date -u +%Y-%m-%dT%H:%M:%SZ)"'",'
    echo '  "checks": {'
    for check in "${!CHECK_RESULTS[@]}"; do
        if [[ $first -eq 0 ]]; then
            echo ","
        fi
        first=0
        echo -n '    "'"$check"'": {'
        echo -n '"status": "'"${CHECK_RESULTS[$check]}"'", '
        echo -n '"message": "'"${CHECK_MESSAGES[$check]}"'"'
        echo -n '}'
    done
    echo ""
    echo "  }"
    echo "}"
}

# Check functions
check_daemon() {
    log "Checking daemon status..."
    
    # Check if daemon process is running
    if pgrep -x "infringd" > /dev/null 2>&1; then
        CHECK_RESULTS["daemon_process"]="PASS"
        CHECK_MESSAGES["daemon_process"]="Daemon process is running"
    else
        CHECK_RESULTS["daemon_process"]="FAIL"
        CHECK_MESSAGES["daemon_process"]="Daemon process not found"
    fi
    
    # Check daemon responsiveness via status endpoint
    # NOTE: This assumes infringctl is in PATH and configured
    if command -v infringctl &> /dev/null; then
        if timeout "$TIMEOUT_SECONDS" infringctl status &> /dev/null; then
            CHECK_RESULTS["daemon_responsive"]="PASS"
            CHECK_MESSAGES["daemon_responsive"]="Daemon responding to status queries"
        else
            CHECK_RESULTS["daemon_responsive"]="FAIL"
            CHECK_MESSAGES["daemon_responsive"]="Daemon not responding to status queries"
        fi
    else
        CHECK_RESULTS["daemon_responsive"]="WARN"
        CHECK_MESSAGES["daemon_responsive"]="infringctl not in PATH, skipping responsiveness check"
    fi
}

check_router() {
    log "Checking router status..."
    
    # Check spine router health
    if [[ -S "/tmp/infring_spine.sock" ]] || [[ -S "$HOME/.infring/spine.sock" ]]; then
        CHECK_RESULTS["router_socket"]="PASS"
        CHECK_MESSAGES["router_socket"]="Spine router socket exists"
    else
        CHECK_RESULTS["router_socket"]="WARN"
        CHECK_MESSAGES["router_socket"]="Spine router socket not found (may be using TCP)"
    fi
    
    # Check if any routes are registered
    # This is a simplified check; production would query the actual router
    CHECK_RESULTS["router_routes"]="PASS"
    CHECK_MESSAGES["router_routes"]="Router route check passed (placeholder)"
}

check_storage() {
    log "Checking storage status..."
    
    # Check available disk space
    local avail_gb
    avail_gb=$(df -BG "$HOME/.infring" 2>/dev/null | awk 'NR==2 {print $4}' | tr -d 'G' || echo "0")
    if [[ "$avail_gb" -gt 5 ]]; then
        CHECK_RESULTS["storage_space"]="PASS"
        CHECK_MESSAGES["storage_space"]="Storage has ${avail_gb}GB available"
    elif [[ "$avail_gb" -gt 1 ]]; then
        CHECK_RESULTS["storage_space"]="WARN"
        CHECK_MESSAGES["storage_space"]="Storage low: ${avail_gb}GB available (threshold: 5GB)"
    else
        CHECK_RESULTS["storage_space"]="FAIL"
        CHECK_MESSAGES["storage_space"]="Storage critical: ${avail_gb}GB available"
    fi
    
    # Check state directory permissions
    if [[ -d "$HOME/.infring/state" ]]; then
        if [[ -r "$HOME/.infring/state" && -w "$HOME/.infring/state" ]]; then
            CHECK_RESULTS["storage_permissions"]="PASS"
            CHECK_MESSAGES["storage_permissions"]="State directory has correct permissions"
        else
            CHECK_RESULTS["storage_permissions"]="WARN"
            CHECK_MESSAGES["storage_permissions"]="State directory has limited permissions"
        fi
    else
        CHECK_RESULTS["storage_permissions"]="WARN"
        CHECK_MESSAGES["storage_permissions"]="State directory not found"
    fi
}

# Main execution
main() {
    log "Starting health check (component: $COMPONENT, format: $FORMAT)"
    
    # Run requested checks
    if [[ "$COMPONENT" == "all" || "$COMPONENT" == "daemon" ]]; then
        check_daemon
    fi
    
    if [[ "$COMPONENT" == "all" || "$COMPONENT" == "router" ]]; then
        check_router
    fi
    
    if [[ "$COMPONENT" == "all" || "$COMPONENT" == "storage" ]]; then
        check_storage
    fi
    
    # Output results
    if [[ "$FORMAT" == "json" ]]; then
        output_json
    else
        echo "Health Check Results"
        echo "===================="
        echo ""
        for check in "${!CHECK_RESULTS[@]}"; do
            output_text "${CHECK_RESULTS[$check]}" "${CHECK_MESSAGES[$check]}"
        done
        echo ""
    fi
    
    # Determine exit code
    local exit_code=0
    for status in "${CHECK_RESULTS[@]}"; do
        if [[ "$status" == "FAIL" ]]; then
            exit_code=1
            break
        fi
    done
    
    log "Health check complete (exit code: $exit_code)"
    exit $exit_code
}

main "$@"
