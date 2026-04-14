import { InfringSdkClient, createInMemoryTransport } from '@infring/sdk';

async function main(): Promise<void> {
  const sdk = new InfringSdkClient({
    transport: createInMemoryTransport({}, { unseeded_behavior: 'synthetic_success' }),
    default_policy_refs: ['policy.runtime.default'],
  });

  const response = await sdk.submitTask({
    prompt: 'Build release checklist for this sprint.',
    metadata: {
      team: 'ops',
      priority: 'high',
    },
  });

  if (!response.ok) {
    throw new Error('submit_task_failed');
  }
}

main().catch((error) => {
  process.stderr.write(`${String(error && (error as Error).message ? (error as Error).message : error)}\n`);
  process.exit(1);
});
