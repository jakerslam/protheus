#!/bin/bash
# Health check utility for Protheus infrastructure
# Performs basic connectivity and service checks

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

echo "=== Protheus Infrastructure Health Check ==="
echo "Check time: $(date)"
echo ""

# Check disk usage
echo "Checking disk usage..."
DISK_USAGE=$(df / | awk 'NR==2 {print $5}' | sed 's/%//')
if [ "$DISK_USAGE" -gt 80 ]; then
    echo -e "${RED}WARNING: Disk usage is ${DISK_USAGE}%${NC}"
else
    echo -e "${GREEN}OK: Disk usage is ${DISK_USAGE}%${NC}"
fi

# Check memory
echo ""
echo "Checking memory..."
if command -v free &> /dev/null; then
    free -h | grep "Mem:"
else
    echo "'free' command not available (macOS?)"
    vm_stat | head -6 || true
fi

# Check load average
echo ""
echo "Checking system load..."
LOAD=$(uptime | awk -F'load averages:' '{print $2}')
echo "Load averages: $LOAD"

echo ""
echo "=== Health check complete ==="
