# Utility Scripts

This directory contains operational utility scripts for the Protheus platform.

## Script Location

Shell utilities were migrated into policy-approved tooling roots:

- `tests/tooling/scripts/utils/root_legacy/log-rotation.sh`
- `tests/tooling/scripts/utils/root_legacy/health-check.sh`

This directory now keeps documentation only.

## Security Notes

- Scripts should run with minimal required privileges
- Log directories should have appropriate permissions (644 for files, 755 for dirs)
- Archive locations should be encrypted at rest

## TODO Items

- [ ] Add backup verification utility
- [ ] Add script for emergency service restarts

---

**Maintained by:** Platform Operations  
**Last Review:** 2026-03-30
