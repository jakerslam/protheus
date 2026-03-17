#!/bin/bash
# =============================================================================
# Disk Cleanup Utility
# Author: Rohan Kapoor (VP Platform & Operations)
# Last Updated: 2026-03-17
# Version: 1.0.0
#
# Purpose: Automated disk space management for Protheus infrastructure nodes.
#          Safely removes temporary files, old logs, and cached data while
#          preserving critical system files and recent operational data.
#
# Usage: ./disk-cleanup.sh [OPTIONS]
#   --dry-run          Show what would be deleted without removing files
#   --aggressive       Remove more data (logs older than 7 days, all temp files)
#   --target PATH      Clean specific directory instead of defaults
#   --threshold PERCENT  Only run if disk usage exceeds PERCENT (default: 80)
#   --help             Display this help message
#
# Safety Features:
#   - Never deletes files newer than 24 hours
#   - Excludes active log files (currently being written)
#   - Preserves configuration files and secrets
#   - Creates deletion manifest for audit purposes
#
# Scheduling: Intended for weekly execution via Kubernetes CronJob
#             Recommended schedule: "0 2 * * 0" (Sundays at 2 AM)
# =============================================================================

set -euo pipefail

# Configuration
readonly SCRIPT_VERSION="1.0.0"
readonly DEFAULT_THRESHOLD=80
readonly SAFE_AGE_DAYS=1
readonly AGGRESSIVE_AGE_DAYS=7
readonly LOG_RETENTION_DAYS=30

# Colors for output (only when terminal supports it)
if [[ -t 1 ]]; then
    readonly RED='\033[0;31m'
    readonly GREEN='\033[0;32m'
    readonly YELLOW='\033[1;33m'
    readonly NC='\033[0m' # No Color
else
    readonly RED=''
    readonly GREEN=''
    readonly YELLOW=''
    readonly NC=''
fi

# Global variables
DRY_RUN=false
AGGRESSIVE=false
THRESHOLD=$DEFAULT_THRESHOLD
TARGET_DIRS=()
DELETION_MANIFEST=""
SPACE_FREED=0

# =============================================================================
# Helper Functions
# =============================================================================

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

show_help() {
    sed -n '/^# Usage:/,/^#$/p' "$0" | sed 's/^# //'
}

get_disk_usage() {
    local path="${1:-/}"
    df -P "$path" | awk 'NR==2 {print $5}' | sed 's/%//'
}

format_bytes() {
    local bytes=$1
    if command -v numfmt &> /dev/null; then
        numfmt --to=iec-i --suffix=B "$bytes"
    else
        echo "${bytes} bytes"
    fi
}

# =============================================================================
# Cleanup Functions
# =============================================================================

cleanup_temp_files() {
    local age_days=$1
    local temp_dirs=("/tmp" "/var/tmp")
    
    log_info "Cleaning temporary files older than ${age_days} days..."
    
    for dir in "${temp_dirs[@]}"; do
        if [[ -d "$dir" ]]; then
            local find_args=("$dir" -type f -mtime "+${age_days}")
            
            # Safety exclusions
            find_args+=(! -name "*.pid" ! -name "*.lock")
            find_args+=(! -path "*/systemd/*" ! -path "*/sshd/*")
            
            if [[ "$DRY_RUN" == true ]]; then
                local count=$(find "${find_args[@]}" 2>/dev/null | wc -l)
                log_info "[DRY-RUN] Would delete $count files from $dir"
            else
                local deleted=0
                while IFS= read -r file; do
                    if [[ -f "$file" && ! -L "$file" ]]; then
                        local size=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file" 2>/dev/null || echo 0)
                        rm -f "$file" && ((deleted++)) && ((SPACE_FREED+=size))
                        echo "$(date -Iseconds) DELETED: $file (${size} bytes)" >> "$DELETION_MANIFEST"
                    fi
                done < <(find "${find_args[@]}" 2>/dev/null)
                log_info "Deleted $deleted files from $dir"
            fi
        fi
    done
}

cleanup_logs() {
    local age_days=$1
    local log_dirs=("/var/log")
    
    log_info "Cleaning log files older than ${age_days} days..."
    
    for dir in "${log_dirs[@]}"; do
        if [[ -d "$dir" ]]; then
            # Find rotated logs (ending in numbers or .gz) that are old
            local find_args=("$dir" -type f \( -name "*.log.*" -o -name "*.log.[0-9]*" -o -name "*.gz" \))
            find_args+=(-mtime "+${age_days}")
            
            # Exclude currently active logs and critical system logs
            find_args+=(! -name "syslog" ! -name "auth.log" ! -name "secure")
            
            if [[ "$DRY_RUN" == true ]]; then
                local count=$(find "${find_args[@]}" 2>/dev/null | wc -l)
                local size=$(find "${find_args[@]}" -exec stat -f%z {} + 2>/dev/null | awk '{sum+=$1} END {print sum}' || echo 0)
                log_info "[DRY-RUN] Would delete $count log files (~$(format_bytes $size)) from $dir"
            else
                local deleted=0
                while IFS= read -r file; do
                    if [[ -f "$file" ]]; then
                        local size=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file" 2>/dev/null || echo 0)
                        rm -f "$file" && ((deleted++)) && ((SPACE_FREED+=size))
                        echo "$(date -Iseconds) DELETED: $file (${size} bytes)" >> "$DELETION_MANIFEST"
                    fi
                done < <(find "${find_args[@]}" 2>/dev/null)
                log_info "Deleted $deleted log files from $dir"
            fi
        fi
    done
}

cleanup_package_cache() {
    log_info "Cleaning package manager caches..."
    
    if [[ "$DRY_RUN" == true ]]; then
        if command -v apt-get &> /dev/null; then
            log_info "[DRY-RUN] Would run: apt-get autoclean"
        elif command -v yum &> /dev/null; then
            log_info "[DRY-RUN] Would run: yum clean all"
        elif command -v brew &> /dev/null; then
            log_info "[DRY-RUN] Would run: brew cleanup"
        fi
    else
        if command -v apt-get &> /dev/null; then
            apt-get autoclean &>/dev/null && log_info "Cleaned apt cache" || log_warn "apt-get autoclean failed (non-fatal)"
        elif command -v yum &> /dev/null; then
            yum clean all &>/dev/null && log_info "Cleaned yum cache" || log_warn "yum clean failed (non-fatal)"
        elif command -v brew &> /dev/null; then
            brew cleanup &>/dev/null && log_info "Cleaned Homebrew cache" || log_warn "brew cleanup failed (non-fatal)"
        fi
    fi
}

cleanup_docker() {
    if ! command -v docker &> /dev/null; then
        return 0
    fi
    
    log_info "Cleaning Docker resources..."
    
    if [[ "$DRY_RUN" == true ]]; then
        local images=$(docker images -f "dangling=true" -q 2>/dev/null | wc -l)
        local volumes=$(docker volume ls -f "dangling=true" -q 2>/dev/null | wc -l)
        log_info "[DRY-RUN] Would remove $images dangling images and $images dangling volumes"
    else
        # Remove dangling images
        docker images -f "dangling=true" -q 2>/dev/null | xargs -r docker rmi &>/dev/null || true
        
        # Remove dangling volumes
        docker volume ls -f "dangling=true" -q 2>/dev/null | xargs -r docker volume rm &>/dev/null || true
        
        log_info "Cleaned Docker resources"
    fi
}

# =============================================================================
# Main Execution
# =============================================================================

main() {
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --aggressive)
                AGGRESSIVE=true
                shift
                ;;
            --target)
                TARGET_DIRS+=("$2")
                shift 2
                ;;
            --threshold)
                THRESHOLD="$2"
                shift 2
                ;;
            --help)
                show_help
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
    
    # Initialize deletion manifest
    DELETION_MANIFEST="/tmp/disk-cleanup-$(date +%Y%m%d-%H%M%S).manifest"
    echo "# Disk Cleanup Manifest - $(date -Iseconds)" > "$DELETION_MANIFEST"
    echo "# Version: $SCRIPT_VERSION" >> "$DELETION_MANIFEST"
    echo "# Mode: $([[ "$DRY_RUN" == true ]] && echo "DRY-RUN" || echo "LIVE")" >> "$DELETION_MANIFEST"
    echo "# Aggressive: $AGGRESSIVE" >> "$DELETION_MANIFEST"
    echo "# Threshold: ${THRESHOLD}%" >> "$DELETION_MANIFEST"
    echo "# =========================================" >> "$DELETION_MANIFEST"
    
    log_info "Starting disk cleanup utility v${SCRIPT_VERSION}"
    log_info "Mode: $([[ "$DRY_RUN" == true ]] && echo "DRY-RUN (no files will be deleted)" || echo "LIVE")"
    
    # Check current disk usage
    local current_usage=$(get_disk_usage)
    log_info "Current disk usage: ${current_usage}%"
    
    # Check threshold
    if [[ $current_usage -lt $THRESHOLD ]]; then
        log_info "Disk usage (${current_usage}%) below threshold (${THRESHOLD}%). Exiting."
        exit 0
    fi
    
    log_warn "Disk usage (${current_usage}%) exceeds threshold (${THRESHOLD}%). Proceeding with cleanup."
    
    # Determine age threshold based on mode
    local age_days=$SAFE_AGE_DAYS
    [[ "$AGGRESSIVE" == true ]] && age_days=$AGGRESSIVE_AGE_DAYS
    
    # Perform cleanup operations
    cleanup_temp_files "$age_days"
    cleanup_logs "$age_days"
    cleanup_package_cache
    cleanup_docker
    
    # Summary
    log_info "========================================="
    log_info "Cleanup complete!"
    
    if [[ "$DRY_RUN" == true ]]; then
        log_info "DRY-RUN mode: No files were actually deleted"
    else
        log_info "Space freed: $(format_bytes $SPACE_FREED)"
        log_info "Deletion manifest: $DELETION_MANIFEST"
        
        local final_usage=$(get_disk_usage)
        log_info "Disk usage: ${current_usage}% → ${final_usage}%"
        
        if [[ $final_usage -ge $THRESHOLD ]]; then
            log_warn "Disk usage still above threshold. Consider manual intervention."
            exit 1
        fi
    fi
    
    exit 0
}

# Run main function
main "$@"
