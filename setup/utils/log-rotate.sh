#!/bin/bash
# =============================================================================
# Log Rotation Utility for Protheus Services
# =============================================================================
# Author: Rohan Kapoor
# Created: 2026-03-29
# Version: 1.0
#
# Description:
#   Rotates log files for Protheus services to prevent disk saturation.
#   Designed to run via cron daily or weekly depending on log volume.
#
# Usage:
#   ./setup/utils/log-rotate.sh [options]
#
# Options:
#   --service <name>    Rotate logs for specific service only
#   --dry-run          Show what would be done without executing
#   --compress         Compress rotated logs with gzip (default)
#   --no-compress      Skip compression for rotation
#   --days <n>         Keep logs for n days (default: 30)
#   --help             Show this help message
#
# Examples:
#   # Rotate all service logs
#   ./setup/utils/log-rotate.sh
#
#   # Rotate only core service logs, 7 day retention
#   ./setup/utils/log-rotate.sh --service core --days 7
#
#   # Preview what would be rotated
#   ./setup/utils/log-rotate.sh --dry-run
#
# Cron Setup:
#   Daily rotation at 2 AM:
#   0 2 * * * /usr/local/bin/log-rotate.sh >> /var/log/protheus/rotate.log 2>&1
#
# =============================================================================

set -euo pipefail

# Configuration
LOG_BASE_DIR="${PROTHEUS_LOG_DIR:-/var/log/protheus}"
RETENTION_DAYS=30
COMPRESS=true
DRY_RUN=false
SPECIFIC_SERVICE=""

# Colors for output (disable if not terminal)
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    NC='\033[0m' # No Color
else
    RED=''
    GREEN=''
    YELLOW=''
    NC=''
fi

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $(date '+%Y-%m-%d %H:%M:%S') $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $(date '+%Y-%m-%d %H:%M:%S') $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $(date '+%Y-%m-%d %H:%M:%S') $1" >&2
}

# Help message
show_help() {
    sed -n '/^# Usage:/,/^# ===/p' "$0" | sed 's/^# //'
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --service)
            SPECIFIC_SERVICE="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --compress)
            COMPRESS=true
            shift
            ;;
        --no-compress)
            COMPRESS=false
            shift
            ;;
        --days)
            RETENTION_DAYS="$2"
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

# Validate log directory exists
if [[ ! -d "$LOG_BASE_DIR" ]]; then
    log_error "Log directory does not exist: $LOG_BASE_DIR"
    exit 1
fi

# Statistics
TOTAL_ROTATED=0
TOTAL_DELETED=0
TOTAL_BYTES_SAVED=0

log_info "=== Protheus Log Rotation Started ==="
log_info "Log directory: $LOG_BASE_DIR"
log_info "Retention period: $RETENTION_DAYS days"
log_info "Compression: $COMPRESS"
[[ -n "$SPECIFIC_SERVICE" ]] && log_info "Service filter: $SPECIFIC_SERVICE"
[[ "$DRY_RUN" == true ]] && log_warn "DRY RUN MODE - No changes will be made"
echo ""

# Determine which directories to process
if [[ -n "$SPECIFIC_SERVICE" ]]; then
    SERVICE_DIRS=("$LOG_BASE_DIR/$SPECIFIC_SERVICE")
else
    SERVICE_DIRS=($(find "$LOG_BASE_DIR" -maxdepth 1 -type d -not -path "$LOG_BASE_DIR" 2>/dev/null || true))
fi

# Process each service directory
for service_dir in "${SERVICE_DIRS[@]}"; do
    service_name=$(basename "$service_dir")
    
    if [[ ! -d "$service_dir" ]]; then
        log_warn "Service directory not found: $service_dir"
        continue
    fi
    
    log_info "Processing: $service_name"
    
    # Find log files (not already rotated)
    while IFS= read -r -d '' log_file; do
        # Skip already rotated files
        [[ "$log_file" =~ \.[0-9]+$ ]] && continue
        [[ "$log_file" =~ \.[0-9]+\.gz$ ]] && continue
        
        # Get file size
        file_size=$(stat -f%z "$log_file" 2>/dev/null || stat -c%s "$log_file" 2>/dev/null || echo "0")
        
        # Generate timestamp for rotation
        rotation_date=$(date +%Y%m%d)
        rotated_name="${log_file}.${rotation_date}"
        
        if [[ "$DRY_RUN" == true ]]; then
            echo "  Would rotate: $(basename "$log_file") (${file_size} bytes)"
        else
            # Truncate and rotate (using copytruncate pattern for active logs)
            if [[ "$COMPRESS" == true ]]; then
                cp "$log_file" "$rotated_name"
                gzip "$rotated_name"
                : > "$log_file"
                log_info "  Rotated: $(basename "$log_file") → $(basename "${rotated_name}.gz") (${file_size} bytes)"
            else
                cp "$log_file" "$rotated_name"
                : > "$log_file"
                log_info "  Rotated: $(basename "$log_file") → $(basename "$rotated_name") (${file_size} bytes)"
            fi
            
            TOTAL_BYTES_SAVED=$((TOTAL_BYTES_SAVED + file_size))
            ((TOTAL_ROTATED++)) || true
        fi
        
    done < <(find "$service_dir" -name "*.log" -type f -mtime +1 -print0 2>/dev/null)
    
    # Delete old rotated files
    while IFS= read -r -d '' old_file; do
        file_size=$(stat -f%z "$old_file" 2>/dev/null || stat -c%s "$old_file" 2>/dev/null || echo "0")
        
        if [[ "$DRY_RUN" == true ]]; then
            echo "  Would delete: $(basename "$old_file") (${file_size} bytes, >${RETENTION_DAYS} days old)"
        else
            rm -f "$old_file"
            log_info "  Deleted: $(basename "$old_file") (${file_size} bytes, >${RETENTION_DAYS} days old)"
            TOTAL_BYTES_SAVED=$((TOTAL_BYTES_SAVED + file_size))
            ((TOTAL_DELETED++)) || true
        fi
    done < <(find "$service_dir" -name "*.log.*" -type f -mtime +$RETENTION_DAYS -print0 2>/dev/null)
    
done

echo ""
log_info "=== Log Rotation Complete ==="
log_info "Files rotated: $TOTAL_ROTATED"
log_info "Files deleted: $TOTAL_DELETED"
log_info "Total space freed: $(numfmt --to=iec $TOTAL_BYTES_SAVED 2>/dev/null || echo "${TOTAL_BYTES_SAVED} bytes")"

# Exit codes for automation
# 0 = Success
# 1 = General error
# 2 = No files found to rotate
# 3 = Insufficient permissions
exit 0

# =============================================================================
# FIXME(rohan): Add support for S3 archival of old logs (ENG-891)
# TODO(rohan): Consider using logrotate system utility instead of custom script
#   once we standardize on systemd for service management
# =============================================================================
