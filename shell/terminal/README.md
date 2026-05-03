# Terminal Shell

Status: initial interactive Shell plug foundation

## Purpose

`shell/terminal/**` is the home for the clean Terminal Shell plug.

The Terminal Shell is a replaceable presentation/input plug that talks through
the Shell Socket contract. It must not import the legacy dashboard, own runtime
truth, or bypass Gateway routes.

## Current Scope

- Socket response test through `ShellSocketGatewayClient`.
- Deterministic fixture mode for CI and local smoke tests.
- Optional live mode against the Gateway/backend route surface at
  `http://127.0.0.1:5173`.
- Infring-branded terminal transcript renderer for bounded projection blocks.
- Gateway launch override via `infring gateway --shell=terminal`.
- Persistent interactive prompt mode. The prompt starts against the Shell
  Socket, selects a Gateway-projected agent row, submits user messages through
  `submit_input`, polls the bounded message window for the agent reply, and
  stops on `Ctrl-Z`.
- Terminal-local Node HTTP transport for live mode, using one closed connection
  per Shell Socket request to avoid stale pooled-fetch failures during Gateway
  restarts.

## Not In Scope Yet

- Session selection.
- Message streaming.
- Rich interactive terminal controls.
- Direct Kernel, Orchestration, or legacy dashboard access.

## Presentation Contract

Terminal rendering starts as plain transcript text:

```text
Infring
ready | agent: Misty | session: current

You
> compare these files

Misty
Thinking...

Tool
read_file | done | 2 files

Misty
Here's the comparison...
```

The renderer is intentionally presentation-only. It accepts bounded Shell
projection blocks and returns text; it does not fetch, cache, decide, mutate
runtime truth, or talk to the legacy dashboard.

## Gateway Launch

Setup writes a shell launch config with a default shell and fallback shell.
`infring gateway` reads that config unless an explicit override is supplied:

```text
infring gateway --shell=ui
infring gateway --shell=terminal
infring gateway --shell=legacy-ui
infring gateway --shell=none
```

Terminal mode suppresses browser auto-open and runs the Terminal Shell through
the Shell Socket live target instead of launching the browser UI. The command
stays attached as an interactive terminal surface until the operator presses
`Ctrl-Z`.
