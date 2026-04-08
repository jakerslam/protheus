import type {
  AttachPoliciesData,
  AttachPoliciesRequest,
  InfringOperation,
  InfringTransport,
  InfringTransportRequest,
  InspectReceiptsData,
  InspectReceiptsRequest,
  JsonObject,
  JsonValue,
  QueryMemoryData,
  QueryMemoryRequest,
  ReviewEvidenceData,
  ReviewEvidenceRequest,
  RunAssimilationData,
  RunAssimilationRequest,
  SdkEnvelope,
  SubmitTaskData,
  SubmitTaskRequest,
} from './types';

function dedupePolicyRefs(rows: string[]): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const row of rows) {
    const normalized = String(row || '').trim();
    if (!normalized || seen.has(normalized)) {
      continue;
    }
    seen.add(normalized);
    out.push(normalized);
  }
  return out;
}

function normalizePayload(input: JsonObject): JsonObject {
  return JSON.parse(JSON.stringify(input)) as JsonObject;
}

export interface InfringSdkClientOptions {
  transport: InfringTransport;
  default_policy_refs?: string[];
}

export class InfringSdkClient {
  private readonly transport: InfringTransport;
  private attachedPolicyRefs: string[];

  constructor(options: InfringSdkClientOptions) {
    this.transport = options.transport;
    this.attachedPolicyRefs = dedupePolicyRefs(options.default_policy_refs || []);
  }

  public getAttachedPolicyRefs(): string[] {
    return [...this.attachedPolicyRefs];
  }

  public async submitTask(request: SubmitTaskRequest): Promise<SdkEnvelope<SubmitTaskData>> {
    return this.invokeOperation<SubmitTaskData>('submit_task', request as unknown as JsonObject);
  }

  public async inspectReceipts(
    request: InspectReceiptsRequest
  ): Promise<SdkEnvelope<InspectReceiptsData>> {
    return this.invokeOperation<InspectReceiptsData>(
      'inspect_receipts',
      request as unknown as JsonObject
    );
  }

  public async queryMemory(request: QueryMemoryRequest): Promise<SdkEnvelope<QueryMemoryData>> {
    return this.invokeOperation<QueryMemoryData>('query_memory', request as unknown as JsonObject);
  }

  public async reviewEvidence(
    request: ReviewEvidenceRequest
  ): Promise<SdkEnvelope<ReviewEvidenceData>> {
    return this.invokeOperation<ReviewEvidenceData>(
      'review_evidence',
      request as unknown as JsonObject
    );
  }

  public async runAssimilation(
    request: RunAssimilationRequest
  ): Promise<SdkEnvelope<RunAssimilationData>> {
    return this.invokeOperation<RunAssimilationData>(
      'run_assimilation',
      request as unknown as JsonObject
    );
  }

  public async attachPolicies(
    request: AttachPoliciesRequest
  ): Promise<SdkEnvelope<AttachPoliciesData>> {
    const refs = (request.policies || [])
      .map((row) => String(row.policy_ref || '').trim())
      .filter((row) => row.length > 0);
    if (String(request.mode || 'merge').toLowerCase() === 'replace') {
      this.attachedPolicyRefs = dedupePolicyRefs(refs);
    } else {
      this.attachedPolicyRefs = dedupePolicyRefs([...this.attachedPolicyRefs, ...refs]);
    }
    const response = await this.invokeOperation<AttachPoliciesData>(
      'attach_policies',
      request as unknown as JsonObject
    );
    if (response.ok && response.data && Array.isArray(response.data.applied_policy_refs)) {
      this.attachedPolicyRefs = dedupePolicyRefs(response.data.applied_policy_refs);
    }
    return response;
  }

  private async invokeOperation<TData extends JsonValue>(
    operation: InfringOperation,
    payload: JsonObject
  ): Promise<SdkEnvelope<TData>> {
    const payloadPolicyRefs = Array.isArray(payload.policy_refs)
      ? payload.policy_refs.map((row) => String(row || '').trim()).filter((row) => row.length > 0)
      : [];
    const effectivePolicyRefs = dedupePolicyRefs([...this.attachedPolicyRefs, ...payloadPolicyRefs]);
    const request: InfringTransportRequest = {
      operation,
      payload: normalizePayload(payload),
      policy_refs: effectivePolicyRefs,
    };
    return this.transport.invoke<TData>(request);
  }
}
