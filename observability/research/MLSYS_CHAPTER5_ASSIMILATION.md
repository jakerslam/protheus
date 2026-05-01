# MLSysBook Chapter 5 Assimilation

Owner: Assurance / Observability
Status: canonical research note

Source: [Machine Learning Systems, Volume 1 - Chapter 5: AI Workflow](https://www.mlsysbook.ai/contents/core/workflow/workflow.html)

## Assimilated Lessons

Chapter 5 frames AI systems as iterative workflows, not linear software delivery. The parts that matter most for InfRing are:

- Feedback loops are primary: monitoring and deployment evidence must influence earlier design and runtime decisions.
- Workflow is not linear: local completion does not imply system-level readiness.
- Validation is continuous: checks belong throughout the lifecycle, not only at release time.
- Production differs from development: clean local metrics can fail under real latency, data quality, resource, and adversarial constraints.
- Systems thinking controls transfer: data quality, runtime constraints, deployment behavior, and monitoring evidence affect each other.

## InfRing Translation

InfRing should treat these lessons as operational constraints:

- `workload_awareness`: Observability must know workload shape, source freshness, volume, latency pressure, and production-like constraints before evidence is promoted.
- `confidence_routing`: Low-confidence evidence routes to probes, worksheets, or validation, not implementation or automatic issue promotion.
- `resource_budgeting`: Feedback loops need report-size, payload, and runtime budgets so monitoring does not become the next outage.
- `dam_diagnosis`: Sentinel findings should move through Diagnose -> Act -> Monitor before closure.

## Guardrail

The executable guard is `ops:mlsys5:assimilation:guard`.

It checks `mlsys_chapter5_workflow_assimilation.json` for source attribution, the five source lessons, all four InfRing translation dimensions, and concrete evidence references for each dimension.
