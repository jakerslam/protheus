# InfRing Installer Surface

The top-level installers are compatibility bootstraps. They must remain boring and shrink over time.

Current policy:

- `install.sh` and `install.ps1` remain the public entrypoints.
- Installer responsibilities should move into named modules under `install/modules/` before large behavior changes are added.
- Repair mode is first-class and must stay guarded.
- Parser/syntax checks are mandatory before release.
- Size growth is treated as installer entropy.

Target shape:

```text
install.sh
install.ps1
install/modules/
  bootstrap_contract.json
  repair_contract.json
  windows_wrapper_contract.json
  platform_detection_contract.json
```

The compatibility installers may call into modules later. Until then, module contracts are the enforcement surface for shrinking and auditing the giant scripts safely.
