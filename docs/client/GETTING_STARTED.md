# Getting Started

InfRing runs with a Rust core and a thin TypeScript surface routed through conduit.

## 1) Install in <2 minutes

### macOS / Linux

```bash
curl -fsSL https://get.protheus.ai/install | sh
infring --help
```

Direct script path:

```bash
curl -fsSL https://get.protheus.ai/install.sh | sh
infring --help
```

Canonical source mirror:

```bash
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh && . "$HOME/.infring/env.sh" && infring gateway
```

If PATH has not refreshed in the same shell, run directly: `~/.infring/bin/infring gateway`.

### Windows (PowerShell)

```powershell
# Use process-scoped bypass so locked-down execution policies do not block install.
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force
$tmp = Join-Path $env:TEMP "infring-install.ps1"
irm https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.ps1 -OutFile $tmp
& $tmp -Repair -Full
# Remove-Item is silent on success in PowerShell.
Remove-Item $tmp -Force
# Confirm command resolution in this shell; if unresolved, use direct-path fallback below.
Get-Command infring -ErrorAction SilentlyContinue
infring gateway
```

If PATH has not refreshed in the same shell, run directly: `$HOME\.protheus\bin\infring.cmd gateway`.

If a release has no Windows prebuilt binary for your architecture, the installer falls back to building from source. Install prerequisites first on fresh machines:

```powershell
winget install --id Git.Git -e
winget install --id Rustlang.Rustup -e
# Optional but often required for MSVC source builds:
winget install --id Microsoft.VisualStudio.2022.BuildTools -e
```

### Optional: Python Wrapper (`pipx`)

```bash
pipx install protheus-cli-wrapper
infring --help
```

## 2) Verify binaries

```bash
infring --help
infringctl --help
infringd --help
```

Legacy aliases remain available with deprecation notices:
- `protheus` -> `infring`
- `protheusctl` -> `infringctl`
- `protheusd` -> `infringd`

## 3) Start core surfaces

```bash
infringd
infring status
infring contract-check
```

## 4) Run mandatory gates

```bash
cargo run --manifest-path core/layer0/ops/Cargo.toml --bin protheus-ops -- contract-check
NODE_PATH=$PWD/node_modules npm run -s formal:invariants:run
```

## Notes

- Rust is the source of truth for kernel logic (primitives, constitution, policy, receipts).
- TypeScript is limited to thin client surfaces and extension/UI workflows via conduit.
- Python is optional and only provides a thin CLI wrapper that forwards to Rust.
