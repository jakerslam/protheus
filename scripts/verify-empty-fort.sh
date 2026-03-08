#!/usr/bin/env bash
set -euo pipefail

MANIFEST="client/docs/community/contributors_manifest.json"
README_FILE="README.md"
RC_FILE=".all-contributorsrc"
MIN_COUNT="${EMPTY_FORT_MIN_COUNT:-100}"
STRICT_README=1

for arg in "$@"; do
  case "$arg" in
    --manifest=*) MANIFEST="${arg#*=}" ;;
    --readme=*) README_FILE="${arg#*=}" ;;
    --contributorsrc=*) RC_FILE="${arg#*=}" ;;
    --min-count=*) MIN_COUNT="${arg#*=}" ;;
    --strict-readme=0) STRICT_README=0 ;;
    --help|-h)
      cat <<USAGE
Usage: scripts/verify-empty-fort.sh [--manifest=...] [--readme=README.md] [--contributorsrc=.all-contributorsrc] [--min-count=100] [--strict-readme=0]
USAGE
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      exit 1
      ;;
  esac
done

python3 - "$MANIFEST" "$RC_FILE" "$README_FILE" "$MIN_COUNT" "$STRICT_README" <<'PY'
import json, re, sys
from pathlib import Path

manifest_path = Path(sys.argv[1])
rc_path = Path(sys.argv[2])
readme_path = Path(sys.argv[3])
min_count = int(sys.argv[4])
strict_readme = int(sys.argv[5])

if not manifest_path.exists():
    raise SystemExit(f'manifest missing: {manifest_path}')
if not rc_path.exists():
    raise SystemExit(f'.all-contributorsrc missing: {rc_path}')
if not readme_path.exists():
    raise SystemExit(f'readme missing: {readme_path}')

manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
contributors = manifest.get('contributors', [])
if not isinstance(contributors, list):
    raise SystemExit('manifest contributors must be an array')

if len(contributors) < min_count:
    raise SystemExit(f'contributor count {len(contributors)} below min {min_count}')

placeholder_re = re.compile(r'^(example|placeholder|test|todo)', re.I)
email_re = re.compile(r'^[^@\s]+@users\.noreply\.github\.com$')
username_re = re.compile(r'^[A-Za-z0-9](?:[A-Za-z0-9-]{0,37}[A-Za-z0-9])?$')

seen = set()
for idx, c in enumerate(contributors):
    login = str(c.get('login', '')).strip()
    if not username_re.match(login):
        raise SystemExit(f'invalid login at index {idx}: {login or "<empty>"}')
    if placeholder_re.search(login):
        raise SystemExit(f'placeholder login detected: {login}')
    if login.lower() in seen:
        raise SystemExit(f'duplicate login in manifest: {login}')
    seen.add(login.lower())
    consent_token = str(c.get('consent_token', '')).strip()
    if not consent_token:
        raise SystemExit(f'missing consent_token for {login}')
    email = str(c.get('email', '')).strip()
    if email and not email_re.match(email):
        raise SystemExit(f'invalid noreply email for {login}: {email}')

rc = json.loads(rc_path.read_text(encoding='utf-8'))
rc_contributors = rc.get('contributors', [])
if len(rc_contributors) != len(contributors):
    raise SystemExit(f'.all-contributorsrc count ({len(rc_contributors)}) != manifest count ({len(contributors)})')

if strict_readme:
    readme = readme_path.read_text(encoding='utf-8')
    if '<!-- EMPTY_FORT:START -->' not in readme or '<!-- EMPTY_FORT:END -->' not in readme:
        raise SystemExit('README missing EMPTY_FORT markers')
    if str(manifest_path) not in readme:
        raise SystemExit('README missing manifest claim-evidence reference')

print(json.dumps({
    'ok': True,
    'manifest': str(manifest_path),
    'contributors': len(contributors),
    'min_count': min_count,
    'strict_readme': bool(strict_readme),
}, indent=2))
PY
