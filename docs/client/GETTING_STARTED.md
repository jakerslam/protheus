# Getting Started

InfRing runs with a Rust core and a thin TypeScript surface routed through conduit.

## 1) Install in <2 minutes

### macOS / Linux

```bash
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh && . "$HOME/.infring/env.sh" && infring gateway
```

If PATH has not refreshed in the same shell, run directly: `~/.infring/bin/infring gateway`.

### Windows (PowerShell)

```powershell
# Use process-scoped bypass so locked-down execution policies do not block install.
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force
$tmp = Join-Path $env:TEMP "infring-install.ps1"
irm https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.ps1 -OutFile $tmp -ErrorAction Stop
& $tmp -Repair -Full
# Remove-Item is silent on success in PowerShell.
Remove-Item $tmp -Force -ErrorAction SilentlyContinue
# Confirm command resolution in this shell; if unresolved, use direct-path fallback below.
Get-Command infring -ErrorAction SilentlyContinue
infring gateway
```

If PATH has not refreshed in the same shell, run directly: `$HOME\.infring\bin\infring.cmd gateway`.

If a release has no Windows prebuilt binary for your architecture, the installer falls back to building from source. Install prerequisites first on fresh machines:

```powershell
winget install --id Git.Git -e
winget install --id Rustlang.Rustup -e
# Optional but often required for MSVC source builds:
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools"
```

When `winget` is unavailable or fails, the installer now attempts a direct Build Tools bootstrapper fallback by default. Disable that path only in locked-down environments:

```powershell
$env:INFRING_INSTALL_ALLOW_DIRECT_MSVC_BOOTSTRAP = "0"
```

### Optional: Python Wrapper (`pipx`)

```bash
pipx install infring-cli-wrapper
infring --help
```

## 2) Verify binaries

```bash
infring --help
infringctl --help
infringd --help
```

Legacy aliases are compatibility-only and should not be used in new automation.

## 3) Start core surfaces

```bash
infringd
infring status
infring contract-check
```

## 4) Run mandatory gates

```bash
cargo run --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops -- contract-check
NODE_PATH=$PWD/node_modules npm run -s formal:invariants:run
```

## Notes

- Rust is the source of truth for kernel logic (primitives, constitution, policy, receipts).
- TypeScript is limited to thin client surfaces and extension/UI workflows via conduit.
- Python is optional and only provides a thin CLI wrapper that forwards to Rust.
