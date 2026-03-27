#!/bin/bash
# log-rotation.sh - Automated log rotation for Protheus services
# Author: Rohan Kapoor <rohan.kapoor@protheuslabs.com>
# Created: 2026-03-17
# Last updated: 2026-03-17
#
# Purpose: Rotate log files to prevent disk space exhaustion while
# maintaining operational visibility. Integrates with existing
# observability stack.
#
# Usage: ./log-rotation.sh [--dry-run] [--verbose]
#   --dry-run   Show what would be done without executing
#   --verbose   Output detailed progress information
#
# Configuration: Edit LOG_DIRS and RETENTION_DAYS below or set via environment
#
# TODO: Add S3 archival integration for long-term storage
# TODO: Consider compressing rotated logs with zstd for better ratio

set -euo pipefail

# Configuration
LOG_DIRS="${LOG_DIRS:-/var/log/protheus $HOME/.protheus/logs}"
RETENTION_DAYS="${RETENTION_DAYS:-30}"
ROTATE_SIZE_MB="${ROTATE_SIZE_MB:-100}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
DRY_RUN=0
VERBOSE=0

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=1
            shift
            ;;
        --verbose)
            VERBOSE=1
            shift
            ;;
        *)
            echo "Unknown option: $1" >&2
            echo "Usage: $0 [--dry-run] [--verbose]" >&2
            exit 1
            ;;
    esac
done

log() {
    if [[ $VERBOSE -eq 1 ]]; then
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*"
    fi
}

rotate_file() {
    local file="$1"
    local size_mb
    size_mb=$(du -m "$file" 2>/dev/null | cut -f1 || echo "0")
    
    if [[ $size_mb -gt $ROTATE_SIZE_MB ]]; then
        local rotated="${file}.${TIMESTAMP}"
        if [[ $DRY_RUN -eq 1 ]]; then
            echo "DRY-RUN: Would rotate $file ($size_mb MB) -> $rotated"
        else
            log "Rotating $file ($size_mb MB)..."
            mv "$file" "$rotated"
            touch "$file"
            chmod 644 "$file"
        fi
    fi
}

main() {
    log "Starting log rotation (retention: ${RETENTION_DAYS} days, size threshold: ${ROTATE_SIZE_MB} MB)"
    
    local total_rotated=0
    
    for dir in $LOG_DIRS; do
        if [[ ! -d "$dir" ]]; then
            log "Skipping $dir (not a directory)"
            continue
        fi
        
        log "Processing directory: $dir"
        
        # Find and rotate large log files
        while IFS= read -r -d '' file; do
            if [[ -f "$file" ]]; then
                rotate_file "$file"
                ((total_rotated++)) || true
            fi
        done < <(find "$dir" -maxdepth 1 -name "*.log" -type f -print0 2>/dev/null)
        
        # Clean up old rotated files
        if [[ $DRY_RUN -eq 1 ]]; then
            echo "DRY-RUN: Would remove files older than $RETENTION_DAYS days in $dir"
        else
            log "Removing files older than $RETENTION_DAYS days..."
            find "$dir" -maxdepth 1 -name "*.log.*" -type f -mtime +$RETENTION_DAYS -delete 2>/dev/null || true
        fi
    done
    
    log "Log rotation complete. Processed $total_rotated files."
}

# Safety check - ensure we're not running as root in production
if [[ $EUID -eq 0 ]] && [[ "${PROTHEUS_ENV:-}" == "production" ]]; then
    echo "WARNING: Running as root in production. Consider using service account." >&2
fi

main "$@"
