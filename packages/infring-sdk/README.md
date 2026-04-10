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

## CLI transport

`createCliTransport` is explicit by design: you provide `args_for_operation` so command mapping stays controlled and auditable per deployment.

```ts
import { InfringSdkClient, createCliTransport } from '@infring/sdk';

const transport = createCliTransport({
  command: 'infring',
  args_for_operation: (req) => [
    'orchestration',
    'invoke',
    `--op=${req.operation}`,
    `--payload-json=${JSON.stringify(req.payload)}`,
  ],
});

const sdk = new InfringSdkClient({ transport });
```

Production channel policy:
- Process transport is emergency-only and blocked for production release channels (`stable`, `prod`, `production`, `ga`, `release`).
- Production deployments must route through the resident IPC topology.
