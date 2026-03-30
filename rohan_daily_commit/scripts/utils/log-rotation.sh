#!/bin/bash
# =================================================================
# Log Rotation Utility
# =================================================================
# Purpose: Automated log rotation for Protheus application logs
# Author: Rohan Kapoor
# Last Updated: 2026-03-30
#
# This script handles rotation of production logs to prevent disk
# space issues while maintaining compliance with retention policies.
#
# Usage: ./log-rotation.sh [--dry-run] [--config=/path/to/config]
# =================================================================

set -euo pipefail

# Configuration
LOG_ROOT="${PROTHEUS_LOG_ROOT:-/var/log/protheus}"
RETENTION_DAYS="${LOG_RETENTION_DAYS:-90}"
COMPRESS_DAYS="${LOG_COMPRESS_DAYS:-7}"
ARCHIVE_BUCKET="${ARCHIVE_BUCKET:-protheus-logs-archive}"

# Color output for terminal
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Logging utilities
log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Dry run mode flag
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --config=*)
            CONFIG_FILE="${1#*=}"
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--dry-run] [--config=/path/to/config]"
            exit 1
            ;;
    esac
done

# Load custom config if provided
if [[ -n "${CONFIG_FILE:-}" && -f "$CONFIG_FILE" ]]; then
    log_info "Loading configuration from $CONFIG_FILE"
    # shellcheck source=/dev/null
    source "$CONFIG_FILE"
fi

# Validate environment
if [[ "$DRY_RUN" == "true" ]]; then
    log_info "Running in DRY-RUN mode. No changes will be made."
fi

# TODO: Add S3 upload functionality for cloud retention
# FIXME: Implement log size-based rotation for high-volume days

log_info "Starting log rotation process..."
log_info "Log root: $LOG_ROOT"
log_info "Retention: $RETENTION_DAYS days"
log_info "Compression threshold: $COMPRESS_DAYS days"

# Function to rotate logs for a specific service
rotate_service_logs() {
    local service_name="$1"
    local service_log_dir="$LOG_ROOT/$service_name"
    
    if [[ ! -d "$service_log_dir" ]]; then
        log_warn "Log directory not found: $service_log_dir"
        return 0
    fi
    
    log_info "Processing logs for service: $service_name"
    
    # Compress logs older than COMPRESS_DAYS
    find "$service_log_dir" -name "*.log" -mtime +$COMPRESS_DAYS -type f | while read -r logfile; do
        if [[ "$DRY_RUN" == "true" ]]; then
            log_info "[DRY-RUN] Would compress: $logfile"
        else
            log_info "Compressing: $logfile"
            gzip -c "$logfile" > "$logfile.gz" && rm "$logfile"
        fi
    done
    
    # Remove compressed logs older than RETENTION_DAYS
    find "$service_log_dir" -name "*.gz" -mtime +$RETENTION_DAYS -type f | while read -r archive; do
        if [[ "$DRY_RUN" == "true" ]]; then
            log_info "[DRY-RUN] Would delete: $archive"
        else
            log_info "Removing old archive: $archive"
            rm "$archive"
        fi
    done
}

# Main execution
main() {
    log_info "Beginning log rotation cycle at $(date -Iseconds)"
    
    # Ensure log root exists
    if [[ ! -d "$LOG_ROOT" ]]; then
        log_error "Log root directory does not exist: $LOG_ROOT"
        exit 1
    fi
    
    # Process each service
    for service_dir in "$LOG_ROOT"/*/; do
        if [[ -d "$service_dir" ]]; then
            service_name=$(basename "$service_dir")
            rotate_service_logs "$service_name"
        fi
    done
    
    # Clean up orphaned compressed files
    log_info "Checking for orphaned archives..."
    # find "$LOG_ROOT" -name "*.gz" -type f -exec ls -lh {} \;
    
    log_info "Log rotation completed at $(date -Iseconds)"
}

main "$@"