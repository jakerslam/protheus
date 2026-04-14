import { InfringSdkClient, createInMemoryTransport } from '@infring/sdk';

async function main(): Promise<void> {
  const sdk = new InfringSdkClient({
    transport: createInMemoryTransport({}, { allow_unseeded_fallback: true }),
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
