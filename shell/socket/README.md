# Shell Socket

Status: canonical Shell 2.0 infrastructure
Owner: Shell presentation contract

## Purpose

`shell/socket/**` is the clean Shell Socket home.

The socket is the stable presentation/input contract that concrete Shell plugs
implement. It is not the legacy dashboard, not a Gateway implementation, and not
a stateful runtime service.

## Directory Contract

- `contract/`: canonical socket capability and projection contracts.
- `client/`: thin transport clients that call Gateway routes only.
- `probe/`: headless or CLI proof harnesses for socket parity.

The named headless capability matrix lives in
[`validation/conformance/contracts/shell_headless_capability_matrix.json`](/Users/jay/.openclaw/workspace/validation/conformance/contracts/shell_headless_capability_matrix.json).
It ties Shell-visible operations to the canonical socket contract, Gateway route
mapping, and browser-free proof commands so the legacy dashboard is not treated
as socket parity evidence.

## Local Route Targets

- `http://127.0.0.1:5173` is the local Gateway/backend route surface used by
  live Shell Socket probes in this workspace.
- `http://127.0.0.1:4173` is the legacy browser host plug. It may proxy API
  traffic, but it is not the canonical Shell Socket target and must not be used
  as proof that a clean shell plug is independent of Shell 1.0.
- The legacy plug quarantine manifest lives at
  [`shell/legacy/legacy_browser_shell_manifest.json`](/Users/jay/.openclaw/workspace/shell/legacy/legacy_browser_shell_manifest.json)
  and is enforced by `ops:shell-socket:legacy:guard`.

Gateway route implementations do not live here. Gateway reads or conforms to the
socket contracts, exposes `/api/shell-socket/**` routes, and forwards accepted
traffic through the appropriate Nexus/Conduit path to Kernel, Orchestration, or
Assurance owners.

## Parity Evidence

The socket has two complementary proof modes:

- `npm run -s ops:shell-socket:headless-probe` proves all socket capabilities
  against a Gateway-shaped fixture without browser assets.
- `npm run -s ops:shell-socket:live-probe` probes the live Gateway target at
  `5173` and reports passed, failed, missing, and exercised capabilities.

The live probe is intentionally stricter than a smoke test: it reports
`live_parity_complete=false` until every required capability is both exercised
and returning bounded projection/ack envelopes. After Shell Socket route changes
in Rust, restart Gateway before treating live probe failures as current runtime
truth.

## Allowed Here

- Shell Socket contracts.
- Typed Shell Socket clients.
- Headless Shell Socket probes.
- Projection-only request/response helpers.

## Not Allowed Here

- Legacy dashboard code.
- Browser framework dependencies.
- Alpine, Svelte, DOM, or localStorage assumptions.
- Kernel, Orchestration, Gateway, or Assurance authority.
- Full runtime mirrors, raw tool payloads, trace bodies, or conversation trees.

Concrete browser, CLI, desktop, mobile, and embedded shells may depend on this
socket. This socket must not depend on any concrete shell.
