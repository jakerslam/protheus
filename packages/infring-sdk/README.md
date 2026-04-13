# `@infring/sdk`

Stable TypeScript SDK contract for the public InfRing surface.

## Stable methods

- `submitTask`
- `inspectReceipts`
- `queryMemory`
- `reviewEvidence`
- `runAssimilation`
- `attachPolicies`

## Design

- Boring by default: one client, typed request/response envelopes.
- Transport is pluggable: CLI transport or in-memory transport.
- Policy refs are first-class and automatically attached to all calls.
- No internal `client/**` or `core/**` imports required by SDK consumers.

## Quick start

```ts
import { InfringSdkClient, createInMemoryTransport } from '@infring/sdk';

const sdk = new InfringSdkClient({
  transport: createInMemoryTransport(),
  default_policy_refs: ['policy.runtime.default'],
});

const task = await sdk.submitTask({
  prompt: 'Summarize the latest receipts for task reliability.',
});

console.log(task.data.task_id, task.receipts[0]?.receipt_id);
```

## Resident IPC transport

Production deployments must route through the resident IPC topology.

```ts
import { InfringSdkClient, createResidentIpcTransport } from '@infring/sdk';

const transport = createResidentIpcTransport({
  invoke: async (req) => residentIpcInvoke(req),
});

const sdk = new InfringSdkClient({ transport });
```

## Dev-only CLI fallback

The old CLI transport lives under the quarantined deep import:

`@infring/sdk/src/transports/cli_dev_only`

It is not exported from the production SDK surface.
