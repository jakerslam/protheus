import fs from "node:fs";
import path from "node:path";

type LanePolicy = {
  id: string;
  purpose: string;
  evidence_paths_any?: string[];
  guard_paths_any?: string[];
};

type Policy = {
  report_path?: string;
  minimum_ready_lanes?: number;
  lanes?: LanePolicy[];
};

const root = process.cwd();
const policyPath = path.join(root, "validation/proof_packs/real_work_workflow_proof_policy.json");
const policy = JSON.parse(fs.readFileSync(policyPath, "utf8")) as Policy;

function firstExisting(paths: string[] = []): string | null {
  for (const rel of paths) {
    if (fs.existsSync(path.join(root, rel))) return rel;
  }
  return null;
}

const lanes = (policy.lanes || []).map((lane) => {
  const evidencePath = firstExisting(lane.evidence_paths_any);
  const guardPath = firstExisting(lane.guard_paths_any);
  const ready = Boolean(evidencePath && guardPath);
  return {
    id: lane.id,
    purpose: lane.purpose,
    ready,
    evidence_path: evidencePath,
    guard_path: guardPath,
    next_action: ready
      ? null
      : `Add fresh evidence and guard coverage for ${lane.id}.`,
  };
});
const readyLanes = lanes.filter((lane) => lane.ready);
const minimumReady = policy.minimum_ready_lanes || 1;
const traceId = `validation:${new Date().toISOString()}:real-work-workflow-proof`;
const report = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: "validation",
  type: "real_work_workflow_proof",
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  ok: readyLanes.length >= minimumReady,
  ready_lane_count: readyLanes.length,
  minimum_ready_lanes: minimumReady,
  total_lane_count: lanes.length,
  lanes,
  summary: readyLanes.length >= minimumReady
    ? "Real-work proof has enough evidence-backed lanes to show the system can convert concrete failures into guarded improvements."
    : "Real-work proof is not ready; too many lanes are missing evidence or guard coverage.",
};
const reportPath = path.join(root, policy.report_path || "core/local/artifacts/real_work_workflow_proof_current.json");
fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify(report, null, 2));
