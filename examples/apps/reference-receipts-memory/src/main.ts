import { InfringSdkClient } from '@infring/sdk';
import { createTestingInMemoryTransport } from '@infring/sdk/testing';

async function main(): Promise<void> {
  const sdk = new InfringSdkClient({
    transport: createTestingInMemoryTransport({
      inspect_receipts: { receipts: [] },
      query_memory: { records: [] },
      review_evidence: { evidence: [] },
    }),
  });

  const receipts = await sdk.inspectReceipts({
    task_id: 'task_demo',
    limit: 20,
  });
  const memory = await sdk.queryMemory({
    query: 'release readiness',
    scope: 'core',
    limit: 10,
  });
  const evidence = await sdk.reviewEvidence({
    task_id: 'task_demo',
    limit: 10,
  });

  if (!receipts.ok || !memory.ok || !evidence.ok) {
    throw new Error('read_contract_failed');
  }
}

main().catch((error) => {
  process.stderr.write(`${String(error && (error as Error).message ? (error as Error).message : error)}\n`);
  process.exit(1);
});
