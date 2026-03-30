# Utility Scripts

This directory contains operational utility scripts for the Protheus platform.

## Available Scripts

### log-rotation.sh

Automates log rotation to manage disk space and comply with retention policies.

**Features:**
- Age-based rotation (compress after 7 days, delete after 90 days)
- Dry-run mode for safe testing
- Per-service log directory handling
- Configurable via environment variables

**Usage:**
```bash
# Standard run
./log-rotation.sh

# Dry run (preview changes)
./log-rotation.sh --dry-run

# With custom config
./log-rotation.sh --config=/etc/protheus/log-rotation.conf
```

## Security Notes

- Scripts should run with minimal required privileges
- Log directories should have appropriate permissions (644 for files, 755 for dirs)
- Archive locations should be encrypted at rest

## TODO Items

- [ ] Add health check script for service monitoring
- [ ] Create backup verification utility
- [ ] Add script for emergency service restarts

---

**Maintained by:** Platform Operations  
**Last Review:** 2026-03-30