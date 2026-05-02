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

Gateway route implementations do not live here. Gateway reads or conforms to the
socket contracts, exposes `/api/shell-socket/**` routes, and forwards accepted
traffic through the appropriate Nexus/Conduit path to Kernel, Orchestration, or
Assurance owners.

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
