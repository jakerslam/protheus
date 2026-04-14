import { InfringSdkClient } from '@infring/sdk';
import { createTestingInMemoryTransport } from '@infring/sdk/testing';

async function main(): Promise<void> {
  const sdk = new InfringSdkClient({
    transport: createTestingInMemoryTransport({}, { unseeded_behavior: 'synthetic_success' }),
  });

  const attached = await sdk.attachPolicies({
    mode: 'replace',
    policies: [
      { policy_ref: 'policy.assimilation.admission.v1', mode: 'enforced' },
      { policy_ref: 'policy.receipts.required.v1', mode: 'enforced' },
    ],
  });

  const assimilation = await sdk.runAssimilation({
    target: 'openclaw_agent_tooling',
    objective: 'assimilate useful agent reliability primitives',
    strict: true,
  });

  if (!attached.ok || !assimilation.ok) {
    throw new Error('assimilation_contract_failed');
  }
}

main().catch((error) => {
  process.stderr.write(`${String(error && (error as Error).message ? (error as Error).message : error)}\n`);
  process.exit(1);
});
