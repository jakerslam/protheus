# Performance Throughput Audit Fix Plan
**Audit ID**: PERF-AUDIT-006  
**Date**: 2026-03-25  
**Status**: Critical - Action Required

---

## Executive Summary

Current system throughput is at **73%** of target (100%), with a **-27% performance gap**. Critical bottlenecks identified in spine success rate (28.6% vs 90% target), receipt latency (412ms p99 vs 400ms target), and cockpit block staleness (87% stale >90s).

---

## Current Performance Metrics (Critical Issues)

| Metric | Current | Target | Gap | Severity |
|--------|---------|--------|-----|----------|
| Performance Throughput | 73% | 100% | -27% | 🔴 Critical |
| Spine Success Rate | 28.6% | 90% | -61.4% | 🔴 Critical |
| Receipt Latency p99 | 412ms | 400ms | +12ms | 🟡 Warning |
| Queue Depth | 44 | <60 | 73% capacity | 🟡 Warning |
| Cockpit Blocks Stale | 29/33 (87%) | <10% | +77% | 🔴 Critical |
| Handoffs/Agent | 0-1616 | Stable | High Variance | 🟡 Warning |

---

## Root Cause Analysis

### 1. Conduit Throughput Bottleneck
**Location**: `core/layer0/ops/src/ops_domain_conduit_runner_kernel.rs`

- **Current**: Fixed 4-signal horizontal scaling
- **Issue**: Insufficient parallel processing for high-throughput workloads
- **Evidence**: Cold start p95 spikes to 16,316ms (see runtime_efficiency_floor_history.jsonl)
- **Impact**: Directly affects spine success rate and receipt latency

### 2. Cockpit Stale Block Accumulation
**Location**: `core/layer0/ops/src/spine.rs` (sleep-cleanup, attention queue)

- **Current**: 87% of cockpit blocks are stale (>90s)
- **Issue**: No automated drain mechanism for stale blocks
- **Impact**: Blocks memory, reduces throughput, causes handoff variance

### 3. Receipt Generation Latency
**Location**: `core/layer0/ops/src/action_receipts_kernel.rs`

- **Current**: Receipt generation adds ~12ms overhead (412ms vs 400ms target)
- **Issue**: Blocking receipt generation in hot path
- **Impact**: P99 latency exceeds target by 3%

### 4. Queue Backpressure Misconfiguration
**Location**: `core/layer0/ops/src/attention_queue.rs`

- **Current**: Queue depth at 44 (73% of 60 threshold)
- **Issue**: Conservative backpressure causing premature throttling
- **Impact**: Reduced effective throughput

---

## Fix Implementation Plan

### Fix 1: Conduit Horizontal Scaling (4 → 6 Signals) 🎯 **HIGH PRIORITY**
**Target**: `core/layer0/ops/src/ops_domain_conduit_runner_kernel.rs`

```rust
// CURRENT: Fixed 4-signal processing
const CONDUIT_SIGNAL_COUNT: usize = 4;

// RECOMMENDED: Scale to 6 signals
const CONDUIT_SIGNAL_COUNT: usize = 6;
const CONDUIT_WORKER_POOL_SIZE: usize = 8;  // Add worker pool
```

**Expected Impact**:
- Spine success rate: 28.6% → 75%+ (+161% improvement)
- Cold start p95: 16,316ms → <8,000ms (50% reduction)
- Throughput: +35% improvement

**Implementation Steps**:
1. Update `ops_domain_conduit_runner_kernel.rs` line ~15-20
2. Add worker pool for parallel signal processing
3. Configure load balancing across 6 signal channels
4. Deploy with canary (25% → 50% → 100%)

---

### Fix 2: Cockpit Stale Block Drain Automation 🎯 **HIGH PRIORITY**
**Target**: `core/layer0/ops/src/spine.rs` (sleep-cleanup policy)

```rust
// CURRENT: Manual cleanup only (SleepCleanupPolicy)
// RECOMMENDED: Automated drain with configurable TTL

const COCKPIT_STALE_BLOCK_TTL_SECS: i64 = 90;
const COCKPIT_DRAIN_BATCH_SIZE: usize = 100;
const COCKPIT_DRAIN_INTERVAL_SECS: i64 = 30;

struct CockpitBlockDrainPolicy {
    enabled: bool,
    stale_threshold_secs: i64,      // 90s
    drain_interval_secs: i64,       // 30s
    batch_size: usize,              // 100 blocks
    max_concurrent_drains: usize,   // 4 threads
}
```

**Expected Impact**:
- Stale blocks: 87% → <5% (-82% reduction)
- Handoff variance: 0-1616 → Stable range
- Memory pressure: Reduced by ~40%

**Implementation Steps**:
1. Add `CockpitBlockDrainPolicy` to spine.rs (~line 60-90)
2. Implement background drain task in `run_sleep_cleanup()`
3. Add metrics for drained blocks
4. Configure via env vars: `PROTHEUS_COCKPIT_DRAIN_ENABLED=1`

---

### Fix 3: Receipt Generation Optimization 🎯 **MEDIUM PRIORITY**
**Target**: `core/layer0/ops/src/action_receipts_kernel.rs`

```rust
// CURRENT: Blocking receipt generation
fn generate_receipt_blocking(...) -> Receipt

// RECOMMENDED: Async batch receipt generation
async fn generate_receipt_batch(
    events: Vec<Event>,
    batch_size: usize,  // 10-20 receipts
) -> Vec<Receipt>

const RECEIPT_BATCH_SIZE: usize = 15;
const RECEIPT_FLUSH_INTERVAL_MS: u64 = 50;
```

**Expected Impact**:
- Receipt latency p99: 412ms → <380ms (8% reduction)
- Throughput: +8% improvement
- CPU utilization: -15% (batching efficiency)

**Implementation Steps**:
1. Add async receipt batching to action_receipts_kernel.rs
2. Implement receipt queue with flush timer
3. Update receipt hash computation to use batching
4. Monitor with `protheus-ops attention-queue status`

---

### Fix 4: Queue Backpressure Tuning 🎯 **MEDIUM PRIORITY**
**Target**: `core/layer0/ops/src/attention_queue.rs`

```rust
// CURRENT: Conservative thresholds
const ATTENTION_MAX_QUEUE_DEPTH: usize = 60;
const BACKPRESSURE_DROP_BELOW: &str = "normal";

// RECOMMENDED: Optimized thresholds
const ATTENTION_MAX_QUEUE_DEPTH: usize = 80;  // +33%
const BACKPRESSURE_DROP_BELOW: &str = "elevated";  // Less aggressive
const ATTENTION_BATCH_SIZE: usize = 50;  // +66%
const ATTENTION_TTL_HOURS: i64 = 2;  // Shorter TTL for freshness
```

**Expected Impact**:
- Queue utilization: 73% → 55% (healthy buffer)
- Throughput: +12% improvement
- Fewer dropped messages under load

**Implementation Steps**:
1. Update `AttentionContract` defaults in attention_queue.rs (~line 30-50)
2. Adjust backpressure logic in `should_apply_backpressure()`
3. Increase batch sizes for queue operations
4. Monitor queue depth via `local/state/ops/attention_queue/status.json`

---

## Performance Improvement Summary

| Fix | Expected Latency Improvement | Expected Throughput Improvement |
|-----|------------------------------|----------------------------------|
| Conduit Scaling (4→6) | -50% cold start p95 | +35% |
| Cockpit Drain Automation | -20% handoff latency | +15% |
| Receipt Optimization | -8% receipt latency | +8% |
| Queue Backpressure | -5% queue wait | +12% |
| **COMBINED** | **-45% p99 latency** | **+70% throughput** |

**Projected Final State**:
- Performance Throughput: 73% → **100%+** (target met)
- Spine Success Rate: 28.6% → **85%+** (target: 90%)
- Receipt Latency p99: 412ms → **<350ms** (target: 400ms ✓)
- Cockpit Stale Blocks: 87% → **<5%** ✓

---

## Implementation Timeline

| Phase | Fixes | Owner | Duration | Start Date |
|-------|-------|-------|----------|------------|
| Phase 1 | Fix 1 (Conduit Scaling) | Core Team | 3 days | 2026-03-26 |
| Phase 2 | Fix 2 (Cockpit Drain) | Core Team | 2 days | 2026-03-29 |
| Phase 3 | Fix 3 (Receipt Opt) | Performance Team | 2 days | 2026-03-31 |
| Phase 4 | Fix 4 (Backpressure) | Performance Team | 1 day | 2026-04-02 |
| Validation | All | QA Team | 2 days | 2026-04-03 |

**Total Timeline**: 10 days (2026-03-26 to 2026-04-06)

---

## Validation Criteria

1. **Spine Success Rate**: Must reach 85%+ for 48 hours consecutive
2. **Receipt Latency p99**: Must stay below 400ms for 24 hours
3. **Cockpit Stale Blocks**: Must be below 10% for 72 hours
4. **Queue Depth**: Must stay below 50 (83% of new 60 limit)
5. **Cold Start p95**: Must stay below 10,000ms

---

## Monitoring & Alerting

Add these metrics to `local/state/ops/metrics/`:

```json
{
  "performance_throughput_realtime": {
    "spine_success_rate_5m": "gauge > 0.85",
    "receipt_latency_p99_5m": "gauge < 400",
    "cockpit_stale_ratio_5m": "gauge < 0.10",
    "queue_depth_current": "gauge < 50"
  }
}
```

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Conduit scaling causes instability | Medium | High | Canary deployment (25%→50%→100%) |
| Cockpit drain loses active blocks | Low | Critical | Drain only blocks >90s + dry-run mode |
| Receipt batching adds latency | Low | Medium | Fallback to sync mode if p99 > 450ms |
| Backpressure causes message drops | Medium | Medium | Gradual threshold adjustment |

---

## Appendix: Files Modified

1. `core/layer0/ops/src/ops_domain_conduit_runner_kernel.rs`
2. `core/layer0/ops/src/spine.rs`
3. `core/layer0/ops/src/action_receipts_kernel.rs`
4. `core/layer0/ops/src/attention_queue.rs`
5. `local/state/ops/runtime_efficiency_floor.json` (monitoring)

---

## Appendix: Benchmark References

- Cold start baseline: 16,125ms p50 (from runtime_efficiency_floor.json)
- Throughput baseline: 2,750 ops/sec (from competitive_benchmark_matrix/latest.json)
- Target cold start: <500ms (policy max_ms)
- Target throughput: 3,750+ ops/sec (100% of competitive matrix parity)

---

*Generated by Performance Throughput Audit Subagent*  
*Session: PERF-AUDIT-006*
