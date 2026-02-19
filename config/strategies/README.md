# Strategy Profiles

Purpose: keep specialized objective/risk policy out of `systems/` code.

Each `*.json` here is declarative policy consumed by generic controllers.

## Minimal Shape

```json
{
  "version": "1.0",
  "id": "default_general",
  "name": "Default General Strategy",
  "status": "active",
  "objective": { "primary": "...", "fitness_metric": "verified_progress_rate", "target_window_days": 14 },
  "risk_policy": { "allowed_risks": ["low"], "max_risk_per_action": 35 },
  "admission_policy": { "allowed_types": [], "blocked_types": [], "max_remediation_depth": 2, "duplicate_window_hours": 24 },
  "ranking_weights": { "composite": 0.35, "actionability": 0.2, "directive_fit": 0.15, "signal_quality": 0.15, "expected_value": 0.1, "risk_penalty": 0.05 },
  "budget_policy": { "daily_runs_cap": 4, "daily_token_cap": 4000, "max_tokens_per_action": 1600 },
  "exploration_policy": { "fraction": 0.25, "every_n": 3, "min_eligible": 3 },
  "stop_policy": { "circuit_breakers": { "http_429_cooldown_hours": 12 } },
  "promotion_policy": { "min_days": 7, "min_attempted": 12, "min_verified_rate": 0.5, "max_reverted_rate": 0.35, "max_stop_ratio": 0.75, "min_shipped": 1 },
  "execution_policy": { "mode": "score_only" },
  "threshold_overrides": {}
}
```

## Selection Rules

1. `AUTONOMY_STRATEGY_ID=<id>` if provided.
2. Otherwise first `status: "active"` profile by filename sort.
3. If none found, controllers fall back to env/default thresholds.

## Notes

- Put use-case/domain-specific strategy logic here.
- Keep `systems/` broadly reusable and strategy-agnostic.
- Keep platform specifics in `skills/` and high-churn shortcuts in `habits/`.
- Recommended rollout: start with `execution_policy.mode = "score_only"` and switch to `"execute"` only after observed stable scorecards.
- Runtime enforcement now uses:
  - `risk_policy.max_risk_per_action` as an admission cap (0-100 risk score scale)
  - `admission_policy.duplicate_window_hours` to suppress rapid retries of equivalent proposal keys
- Strict validation blocks profiles with contradictory admission lists (`allowed_types` intersect `blocked_types`) or invalid promotion policy (`min_shipped > min_attempted`).
