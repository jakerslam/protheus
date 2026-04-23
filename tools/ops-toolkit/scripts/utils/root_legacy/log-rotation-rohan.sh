#!/bin/bash
#
# Log Rotation Script
# 
# Purpose: Automated log rotation for Infring application logs
# Author: Rohan Kapoor
# Date: April 8, 2026
#
# This script handles rotation of application logs to prevent disk space issues
# while maintaining compliance requirements for log retention.
#

set -euo pipefail

# Configuration
LOG_DIR="${LOG_DIR:-/var/log/infring}"
ARCHIVE_DIR="${ARCHIVE_DIR:-/var/log/infring/archive}"
RETENTION_DAYS="${RETENTION_DAYS:-90}"
MAX_LOG_SIZE="${MAX_LOG_SIZE:-1G}"
DATE_FORMAT="$(date +%Y%m%d_%H%M%S)"
SCRIPT_NAME="$(basename "$0")"

# Logging function
log_info() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [INFO] $SCRIPT_NAME: $*"
}

log_warn() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [WARN] $SCRIPT_NAME: $*" >&2
}

log_error() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [ERROR] $SCRIPT_NAME: $*" >&2
}

# TODO(rkapoor): Consider adding compressed size validation after gzip operations
# to detect corruption issues early. Could compare pre/post sizes and verify
# archive integrity. Tracked in INFRA-2852.

# Ensure archive directory exists
setup_directories() {
    if [[ ! -d "$ARCHIVE_DIR" ]]; then
        log_info "Creating archive directory: $ARCHIVE_DIR"
        mkdir -p "$ARCHIVE_DIR"
    fi
}

# Rotate logs that exceed size threshold
rotate_large_logs() {
    log_info "Checking for logs exceeding ${MAX_LOG_SIZE}..."
    
    find "$LOG_DIR" -maxdepth 1 -type f -name "*.log" -size "+${MAX_LOG_SIZE}" -print0 | \
    while IFS= read -r -d '' logfile; do
        local filename
        filename=$(basename "$logfile")
        local rotated_name="${ARCHIVE_DIR}/${filename}.${DATE_FORMAT}"
        
        log_info "Rotating large log: $filename"
        
        # Copy to archive with timestamp
        cp "$logfile" "$rotated_name"
        gzip "$rotated_name"
        
        # Truncate original (safer than delete for open file handles)
        truncate -s 0 "$logfile"
        
        log_info "Rotated: $filename -> ${rotated_name}.gz"
    done
}

# Archive old logs (older than retention period)
archive_old_logs() {
    log_info "Archiving logs older than ${RETENTION_DAYS} days..."
    
    local archived_count=0
    
    # Find and compress logs older than retention period
    find "$LOG_DIR" -maxdepth 1 -type f -name "*.log" -mtime "+${RETENTION_DAYS}" -print0 | \
    while IFS= read -r -d '' logfile; do
        local filename
        filename=$(basename "$logfile")
        local archive_path="${ARCHIVE_DIR}/${filename}.${DATE_FORMAT}.gz"
        
        log_info "Archiving old log: $filename"
        gzip -c "$logfile" > "$archive_path"
        rm "$logfile"
        
        ((archived_count++)) || true
        log_info "Archived: $filename"
    done
    
    log_info "Archived $archived_count old log files"
}

# Clean up expired archives
cleanup_old_archives() {
    log_info "Cleaning up archives older than ${RETENTION_DAYS} days..."
    
    local deleted_count=0
    
    find "$ARCHIVE_DIR" -type f -name "*.gz" -mtime "+${RETENTION_DAYS}" -print0 | \
    while IFS= read -r -d '' archive; do
        log_info "Removing expired archive: $(basename "$archive")"
        rm -f "$archive"
        ((deleted_count++)) || true
    done
    
    log_info "Removed $deleted_count expired archives"
}

# Check disk space after rotation
check_disk_space() {
    local usage
    usage=$(df -h "$LOG_DIR" | awk 'NR==2 {print $5}' | tr -d '%')
    
    if [[ "$usage" -gt 85 ]]; then
        log_warn "Disk usage is ${usage}% - consider adjusting retention policies"
    else
        log_info "Current disk usage: ${usage}%"
    fi
}

# Main execution
main() {
    log_info "Starting log rotation process"
    
    # Validate log directory
    if [[ ! -d "$LOG_DIR" ]]; then
        log_error "Log directory does not exist: $LOG_DIR"
        exit 1
    fi
    
    setup_directories
    rotate_large_logs
    archive_old_logs
    cleanup_old_archives
    check_disk_space
    
    log_info "Log rotation completed successfully"
}

# Run main function
main "$@"
