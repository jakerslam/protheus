#!/bin/bash
#
# Infring Log Analysis Utility
# Author: Rohan Kapoor
# Version: 1.0.0
# Date: April 15, 2026
#
# Description:
#   Parses and analyzes Infring application logs to extract useful metrics
#   and identify patterns. Useful for debugging, performance analysis, and
#   generating operational reports.
#
# Usage:
#   ./log-analyzer.sh [options] <log_file>
#   ./log-analyzer.sh --summary /var/log/infring/app.log
#   ./log-analyzer.sh --errors --since="24 hours ago" /var/log/infring/app.log
#
# Exit Codes:
#   0 - Analysis completed successfully
#   1 - No log file provided or file not found
#   2 - Invalid options specified
#

set -o pipefail

# Configuration
readonly SCRIPT_VERSION="1.0.0"
readonly DEFAULT_LOG_DIR="/var/log/infring"

# Analysis modes
MODE="summary"
SINCE=""
UNTIL=""
OUTPUT_FORMAT="text"
TOP_N=10

# Colors for output
if [[ -t 1 ]]; then
    readonly RED='\033[0;31m'
    readonly GREEN='\033[0;32m'
    readonly YELLOW='\033[1;33m'
    readonly BLUE='\033[0;34m'
    readonly NC='\033[0m'
else
    readonly RED=''
    readonly GREEN=''
    readonly YELLOW=''
    readonly BLUE=''
    readonly NC=''
fi

# Logging functions
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }

# Validate log file exists
validate_log_file() {
    local log_file="$1"
    
    if [[ -z "$log_file" ]]; then
        log_error "No log file specified"
        return 1
    fi
    
    if [[ ! -f "$log_file" ]]; then
        log_error "Log file not found: $log_file"
        return 1
    fi
    
    if [[ ! -r "$log_file" ]]; then
        log_error "Cannot read log file (permission denied): $log_file"
        return 1
    fi
    
    return 0
}

# Extract timestamp from log line (assumes ISO8601 format)
extract_timestamp() {
    local line="$1"
    echo "$line" | grep -oE '[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}' | head -1
}

# Summary analysis - overview of log content
analyze_summary() {
    local log_file="$1"
    
    echo "=========================================="
    echo "Log Analysis Summary"
    echo "=========================================="
    echo "Log File: $log_file"
    echo "Generated: $(date -Iseconds)"
    echo ""
    
    # Total lines
    local total_lines
    total_lines=$(wc -l < "$log_file")
    echo "Total Lines: $total_lines"
    
    # File size
    local file_size
    file_size=$(du -h "$log_file" | cut -f1)
    echo "File Size: $file_size"
    
    # Time range
    local first_ts last_ts
    first_ts=$(head -1 "$log_file" | extract_timestamp)
    last_ts=$(tail -1 "$log_file" | extract_timestamp)
    echo "Time Range: ${first_ts:-"unknown"} to ${last_ts:-"unknown"}"
    echo ""
    
    # Log level distribution
    echo "Log Level Distribution:"
    grep -oE '\[(ERROR|WARN|INFO|DEBUG)\]' "$log_file" 2>/dev/null | sort | uniq -c | sort -rn | sed 's/^/  /'
    
    echo ""
    echo "Top 10 Log Sources:"
    grep -oE '\[.*\]:' "$log_file" 2>/dev/null | sort | uniq -c | sort -rn | head -10 | sed 's/^/  /'
}

# Error analysis - focus on ERROR level entries
analyze_errors() {
    local log_file="$1"
    
    echo "=========================================="
    echo "Error Analysis Report"
    echo "=========================================="
    
    local error_count
    error_count=$(grep -c '\[ERROR\]' "$log_file" 2>/dev/null || echo "0")
    echo "Total ERROR entries: $error_count"
    echo ""
    
    if [[ "$error_count" -gt 0 ]]; then
        echo "Most Common Error Patterns:"
        grep '\[ERROR\]' "$log_file" 2>/dev/null | sed 's/.*\[ERROR\] //' | sort | uniq -c | sort -rn | head -"$TOP_N" | sed 's/^/  /'
        
        echo ""
        echo "Recent Errors (last 5):"
        grep '\[ERROR\]' "$log_file" 2>/dev/null | tail -5 | sed 's/^/  /'
    else
        echo "No ERROR entries found in log file."
    fi
}

# Performance analysis - extract timing metrics
analyze_performance() {
    local log_file="$1"
    
    echo "=========================================="
    echo "Performance Analysis Report"
    echo "=========================================="
    
    # Look for timing-related log entries
    local timing_entries
    timing_entries=$(grep -cE '(completed in|duration|latency|took)' "$log_file" 2>/dev/null || echo "0")
    echo "Timing metrics found: $timing_entries entries"
    echo ""
    
    if [[ "$timing_entries" -gt 0 ]]; then
        echo "Sample timing entries:"
        grep -E '(completed in|duration|latency|took)' "$log_file" 2>/dev/null | tail -10 | sed 's/^/  /'
    fi
    
    echo ""
    echo "High-frequency operations (potential optimization candidates):"
    grep -oE '\[.*\]:' "$log_file" 2>/dev/null | sort | uniq -c | sort -rn | head -5 | sed 's/^/  /'
}

# Usage information
usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS] <log_file>

Options:
    -h, --help              Show this help message
    -m, --mode MODE         Analysis mode: summary|errors|performance (default: summary)
    -s, --since DATE        Only analyze entries since date (e.g., "24 hours ago")
    -u, --until DATE        Only analyze entries until date
    -n, --top N             Show top N results (default: 10)
    --version               Show script version

Examples:
    $(basename "$0") /var/log/infring/app.log
    $(basename "$0") -m errors -s "24 hours ago" /var/log/infring/app.log
    $(basename "$0") -m performance -n 20 /var/log/infring/app.log

EOF
}

# Parse arguments
parse_args() {
    local log_file=""
    
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -h|--help)
                usage
                exit 0
                ;;
            -m|--mode)
                MODE="$2"
                shift 2
                ;;
            -s|--since)
                SINCE="$2"
                shift 2
                ;;
            -u|--until)
                UNTIL="$2"
                shift 2
                ;;
            -n|--top)
                TOP_N="$2"
                shift 2
                ;;
            --version)
                echo "Version: $SCRIPT_VERSION"
                exit 0
                ;;
            -*)
                log_error "Unknown option: $1"
                usage
                exit 2
                ;;
            *)
                log_file="$1"
                shift
                ;;
        esac
    done
    
    validate_log_file "$log_file" || exit 1
    
    # Return log file path
    echo "$log_file"
}

# Main execution
main() {
    local log_file
    log_file=$(parse_args "$@")
    
    log_info "Starting log analysis (mode: $MODE)"
    log_info "Target file: $log_file"
    
    case "$MODE" in
        summary)
            analyze_summary "$log_file"
            ;;
        errors)
            analyze_errors "$log_file"
            ;;
        performance)
            analyze_performance "$log_file"
            ;;
        *)
            log_error "Unknown analysis mode: $MODE"
            usage
            exit 2
            ;;
    esac
    
    echo ""
    log_success "Analysis complete"
}

# Run main function
main "$@"
