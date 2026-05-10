# Installer Modules

These modules are the shrink target for the public installers.

`install.sh` and `install.ps1` remain public compatibility bootstraps, but new installer behavior should land here first as a named, testable module. The long-term goal is that the top-level installers only parse arguments, load modules, and dispatch bounded steps.

Current modules:

- `windows_wrappers.ps1`: creates and verifies Windows `.cmd` / `.ps1` command wrappers.
- `completion_card.ps1`: prints the install completion card in a consistent operator-facing format.
- `bootstrap_common.sh`: shared POSIX bootstrap helpers for status lines, path creation, and completion output.
