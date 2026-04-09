export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | JsonObject | JsonValue[];
export type JsonObject = { [key: string]: JsonValue };

export type InfringOperation =
  | 'submit_task'
  | 'inspect_receipts'
  | 'query_memory'
  | 'review_evidence'
  | 'run_assimilation'
  | 'attach_policies';

export interface ReceiptPointer {
  receipt_id: string;
  issued_at: string;
  policy_ref?: string;
}

export interface PolicyRef {
  policy_ref: string;
  revision?: string;
  mode?: 'advisory' | 'enforced';
}

export interface SdkEnvelope<TData extends JsonValue = JsonValue> {
  ok: boolean;
  operation: InfringOperation;
  trace_id: string;
  receipts: ReceiptPointer[];
  data: TData;
  error?: {
    code: string;
    message: string;
  };
}

export interface SubmitTaskRequest {
  prompt: string;
  task_id?: string;
  metadata?: JsonObject;
  policy_refs?: string[];
}

export interface SubmitTaskData {
  task_id: string;
  accepted: boolean;
  status: 'queued' | 'running' | 'blocked';
}

export interface InspectReceiptsRequest {
  task_id?: string;
  receipt_ids?: string[];
  since_ts?: string;
  limit?: number;
  policy_refs?: string[];
}

export interface InspectReceiptsData {
  receipts: ReceiptPointer[];
  next_cursor?: string;
}

export interface QueryMemoryRequest {
  query: string;
  scope?: string;
  limit?: number;
  filters?: JsonObject;
  policy_refs?: string[];
}

export interface MemoryRecord {
  memory_id: string;
  summary: string;
  source_ref?: string;
  score?: number;
}

export interface QueryMemoryData {
  records: MemoryRecord[];
}

export interface ReviewEvidenceRequest {
  task_id?: string;
  query?: string;
  evidence_ids?: string[];
  limit?: number;
  policy_refs?: string[];
}

export interface EvidenceRecord {
  evidence_id: string;
  summary: string;
  source_ref?: string;
  confidence?: number;
}

export interface ReviewEvidenceData {
  evidence: EvidenceRecord[];
}

export interface RunAssimilationRequest {
  target: string;
  objective?: string;
  strict?: boolean;
  policy_refs?: string[];
}

export interface RunAssimilationData {
  assimilation_id: string;
  admitted: boolean;
  status: 'planned' | 'running' | 'completed' | 'rejected';
}

export interface AttachPoliciesRequest {
  policies: PolicyRef[];
  mode?: 'merge' | 'replace';
}

export interface AttachPoliciesData {
  applied_policy_refs: string[];
}

export interface InfringTransportRequest {
  operation: InfringOperation;
  payload: JsonObject;
  policy_refs: string[];
}

export interface InfringTransport {
  invoke<TData extends JsonValue = JsonValue>(
    request: InfringTransportRequest
  ): Promise<SdkEnvelope<TData>>;
}
