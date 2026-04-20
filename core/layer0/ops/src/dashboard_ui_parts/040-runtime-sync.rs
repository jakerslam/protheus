fn build_runtime_sync(root: &Path, flags: &Flags) -> Value {
    let team = if flags.team.trim().is_empty() {
        DEFAULT_TEAM.to_string()
    } else {
        clean_text(&flags.team, 80)
    };

    let cockpit = run_lane(
        root,
        "hermes-plane",
        &[
            "cockpit".to_string(),
            format!("--max-blocks={RUNTIME_SYNC_MAX_BLOCKS}"),
            "--strict=1".to_string(),
        ],
    );
    let attention_status = run_lane(root, "attention-queue", &["status".to_string()]);
    let attention_next = run_lane(
        root,
        "attention-queue",
        &[
            "next".to_string(),
            "--consumer=dashboard_mirror".to_string(),
            "--limit=32".to_string(),
            "--wait-ms=0".to_string(),
            "--run-context=dashboard_mirror".to_string(),
        ],
    );

    let cockpit_payload = cockpit.payload.unwrap_or_else(|| json!({}));
    let attention_status_payload = attention_status.payload.unwrap_or_else(|| json!({}));
    let attention_next_payload = attention_next.payload.unwrap_or_else(|| json!({}));

    let blocks = cockpit_payload
        .pointer("/cockpit/render/stream_blocks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .take(RUNTIME_SYNC_MAX_BLOCKS)
        .collect::<Vec<_>>();

    let cockpit_metrics = cockpit_payload
        .pointer("/cockpit/metrics")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let stale_threshold_ms = i64_from_value(
        cockpit_metrics.get("stale_block_threshold_ms"),
        RUNTIME_SYNC_STALE_BLOCK_MS,
    );
    let blocks = blocks
        .into_iter()
        .map(|mut row| {
            let duration = i64_from_value(row.get("duration_ms"), 0);
            let stale = row
                .get("is_stale")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || duration >= stale_threshold_ms;
            let sequence = clean_text(
                row.get("receipt_hash")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                160,
            );
            row["freshness"] = json!({
                "source": "cockpit_block_receipt",
                "sequence": if sequence.is_empty() {
                    Value::String(crate::deterministic_receipt_hash(&row))
                } else {
                    Value::String(sequence)
                },
                "age_ms": duration.max(0),
                "stale": stale
            });
            row
        })
        .collect::<Vec<_>>();
    let stale_dormant_threshold_ms = 6 * 60 * 60 * 1000;
    let mut duration_values = Vec::<i64>::new();
    let mut status_counts = HashMap::<String, i64>::new();
    let mut lane_counts_map = HashMap::<String, i64>::new();
    let mut stale_actionable_by_lane = HashMap::<String, i64>::new();
    let mut stale_dormant_by_lane = HashMap::<String, i64>::new();
    let mut stale_measured_raw = 0i64;
    for row in &blocks {
        let duration = i64_from_value(row.get("duration_ms"), 0);
        duration_values.push(duration);

        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 40)
            .to_ascii_lowercase();
        if !status.is_empty() {
            *status_counts.entry(status).or_insert(0) += 1;
        }

        let lane = clean_text(row.get("lane").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let lane_key = if lane.is_empty() {
            "unknown".to_string()
        } else {
            lane
        };
        *lane_counts_map.entry(lane_key.clone()).or_insert(0) += 1;

        let stale_flag = row
            .get("is_stale")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || duration >= stale_threshold_ms;
        if stale_flag {
            stale_measured_raw += 1;
            if duration >= stale_dormant_threshold_ms {
                *stale_dormant_by_lane.entry(lane_key).or_insert(0) += 1;
            } else {
                *stale_actionable_by_lane.entry(lane_key).or_insert(0) += 1;
            }
        }
    }
    let total_block_count_value = cockpit_metrics
        .get("total_block_count")
        .cloned()
        .or_else(|| {
            cockpit_payload
                .pointer("/cockpit/render/total_blocks")
                .cloned()
        });
    let total_block_count = i64_from_value(total_block_count_value.as_ref(), blocks.len() as i64)
        .max(blocks.len() as i64);
    let stale_from_metrics =
        i64_from_value(cockpit_metrics.get("stale_block_count"), stale_measured_raw);
    let stale_block_raw_count = stale_measured_raw.max(stale_from_metrics);
    let stale_block_dormant_count = stale_dormant_by_lane
        .values()
        .copied()
        .sum::<i64>()
        .min(stale_block_raw_count);
    let stale_block_count = stale_block_raw_count.saturating_sub(stale_block_dormant_count);
    let active_block_count = (total_block_count - stale_block_raw_count).max(0);

    let mut sorted_durations = duration_values.clone();
    sorted_durations.sort_unstable();
    let duration_sum = duration_values.iter().sum::<i64>();
    let duration_avg = if duration_values.is_empty() {
        0
    } else {
        duration_sum / duration_values.len() as i64
    };
    let duration_max = sorted_durations.last().copied().unwrap_or(0);
    let duration_p95 = if sorted_durations.is_empty() {
        0
    } else {
        let idx = (((sorted_durations.len() as f64) * 0.95).ceil() as usize)
            .saturating_sub(1)
            .min(sorted_durations.len() - 1);
        sorted_durations[idx]
    };

    let mut status_counts_json = serde_json::Map::<String, Value>::new();
    let mut status_count_rows = status_counts.into_iter().collect::<Vec<_>>();
    status_count_rows.sort_by(|a, b| a.0.cmp(&b.0));
    for (key, value) in status_count_rows {
        status_counts_json.insert(key, json!(value));
    }

    let mut lane_counts_json = serde_json::Map::<String, Value>::new();
    let mut lane_count_rows = lane_counts_map.into_iter().collect::<Vec<_>>();
    lane_count_rows.sort_by(|a, b| a.0.cmp(&b.0));
    for (key, value) in lane_count_rows {
        lane_counts_json.insert(key, json!(value));
    }

    let lane_top_rows = |map: &HashMap<String, i64>| -> Vec<Value> {
        let mut rows = map
            .iter()
            .map(|(lane, count)| (lane.clone(), *count))
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        rows.into_iter()
            .take(8)
            .map(|(lane, count)| json!({"lane": lane, "count": count}))
            .collect::<Vec<_>>()
    };
    let stale_lanes_top = lane_top_rows(&stale_actionable_by_lane);
    let stale_lanes_dormant_top = lane_top_rows(&stale_dormant_by_lane);

    let mut slowest_rows = blocks.clone();
    slowest_rows.sort_by_key(|row| Reverse(i64_from_value(row.get("duration_ms"), 0)));
    let slowest_blocks = slowest_rows
        .into_iter()
        .take(8)
        .map(|row| {
            json!({
                "lane": clean_text(row.get("lane").and_then(Value::as_str).unwrap_or(""), 80),
                "event_type": clean_text(row.get("event_type").and_then(Value::as_str).unwrap_or(""), 120),
                "duration_ms": i64_from_value(row.get("duration_ms"), 0),
                "status": clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 40),
                "is_stale": row.get("is_stale").and_then(Value::as_bool).unwrap_or(false),
                "ts": clean_text(row.get("ts").and_then(Value::as_str).unwrap_or(""), 80),
                "path": clean_text(row.get("path").and_then(Value::as_str).unwrap_or(""), 200)
            })
        })
        .collect::<Vec<_>>();

    let mut trend_rows = blocks.clone();
    trend_rows.sort_by(|a, b| {
        clean_text(a.get("ts").and_then(Value::as_str).unwrap_or(""), 80).cmp(&clean_text(
            b.get("ts").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    let trend_start = trend_rows.len().saturating_sub(24);
    let trend = trend_rows
        .into_iter()
        .skip(trend_start)
        .map(|row| {
            json!({
                "ts": clean_text(row.get("ts").and_then(Value::as_str).unwrap_or(""), 80),
                "lane": clean_text(row.get("lane").and_then(Value::as_str).unwrap_or(""), 80),
                "duration_ms": i64_from_value(row.get("duration_ms"), 0),
                "is_stale": row.get("is_stale").and_then(Value::as_bool).unwrap_or(false)
            })
        })
        .collect::<Vec<_>>();

    let conduit_detected_from_blocks = blocks
        .iter()
        .filter(|row| {
            let lane = row
                .get("lane")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            let event_type = row
                .get("event_type")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            lane.contains("conduit")
                || event_type.contains("conduit")
                || row
                    .get("conduit_enforced")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        .count() as i64;
    let conduit_signals = i64_from_value(
        cockpit_metrics
            .get("conduit_signals_active")
            .or_else(|| cockpit_metrics.get("conduit_signals")),
        conduit_detected_from_blocks,
    );
    let conduit_channels_total = i64_from_value(
        cockpit_metrics.get("conduit_signals_total"),
        conduit_detected_from_blocks.max(conduit_signals),
    );
    let conduit_channels_observed = i64_from_value(
        cockpit_metrics.get("conduit_channels_observed"),
        conduit_signals,
    );
    let cockpit_to_conduit_ratio = if conduit_signals > 0 {
        total_block_count as f64 / conduit_signals as f64
    } else {
        total_block_count as f64
    };

    let queue_depth = i64_from_value(
        attention_status_payload
            .get("queue_depth")
            .or_else(|| attention_next_payload.get("queue_depth")),
        0,
    );
    let attention_contract = attention_status_payload
        .get("attention_contract")
        .and_then(Value::as_object)
        .or_else(|| {
            attention_next_payload
                .get("attention_contract")
                .and_then(Value::as_object)
        })
        .cloned()
        .unwrap_or_default();
    let max_queue_depth = i64_from_value(attention_contract.get("max_queue_depth"), 2048).max(1);
    let backpressure_soft_watermark = i64_from_value(
        attention_contract.get("backpressure_soft_watermark"),
        ((max_queue_depth as f64 * 0.75).ceil() as i64).max(1),
    )
    .clamp(1, max_queue_depth);
    let backpressure_hard_watermark = i64_from_value(
        attention_contract.get("backpressure_hard_watermark"),
        max_queue_depth,
    )
    .clamp(backpressure_soft_watermark, max_queue_depth);
    let queue_utilization = (queue_depth as f64 / max_queue_depth as f64).clamp(0.0, 1.0);
    let active_agents = i64_from_value(cockpit_metrics.get("active_agent_count"), 0);
    let target_conduit_signals = recommended_conduit_signals(
        queue_depth,
        queue_utilization,
        conduit_channels_observed,
        active_agents,
    );
    let conduit_scale_required = conduit_channels_observed < target_conduit_signals;
    let sync_mode = if queue_depth >= RUNTIME_SYNC_BATCH_DEPTH {
        "batch_sync"
    } else if queue_depth >= RUNTIME_SYNC_DELTA_DEPTH {
        "delta_sync"
    } else {
        "live_sync"
    };
    let pressure_level = if queue_depth >= backpressure_hard_watermark || queue_utilization >= 0.90
    {
        "critical"
    } else if queue_depth >= backpressure_soft_watermark
        || queue_depth >= RUNTIME_SYNC_BATCH_DEPTH
        || queue_utilization >= 0.75
    {
        "high"
    } else if queue_depth >= RUNTIME_SYNC_WARN_DEPTH || queue_utilization >= 0.60 {
        "elevated"
    } else {
        "normal"
    };

    let events = attention_next_payload
        .get("events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut critical_events_full = Vec::<Value>::new();
    let mut telemetry_events = Vec::<Value>::new();
    let mut standard_events = Vec::<Value>::new();
    let mut background_events = Vec::<Value>::new();
    for row in &events {
        let lane = row
            .get("priority_lane")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        let event_type = row
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        let severity = row
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if lane == "telemetry" || event_type.contains("telemetry") {
            telemetry_events.push(row.clone());
        } else if lane == "critical" || severity == "critical" || severity == "error" {
            critical_events_full.push(row.clone());
        } else if lane == "background" || severity == "background" {
            background_events.push(row.clone());
        } else {
            standard_events.push(row.clone());
        }
    }
    let critical_visible_count = critical_events_full.len() as i64;
    let telemetry_count = telemetry_events.len() as i64;
    let standard_count = standard_events.len() as i64;
    let background_count = background_events.len() as i64;
    let lane_counts = attention_status_payload
        .get("lane_counts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let critical_total_count = i64_from_value(lane_counts.get("critical"), critical_visible_count)
        .max(critical_visible_count);
    let telemetry_total_count =
        i64_from_value(lane_counts.get("telemetry"), telemetry_count).max(telemetry_count);
    let standard_total_count = i64_from_value(lane_counts.get("standard"), standard_count);
    let background_total_count = i64_from_value(lane_counts.get("background"), background_count);
    let critical_events = critical_events_full
        .iter()
        .take(16)
        .cloned()
        .collect::<Vec<_>>();
    let telemetry_micro_batches = attention_next_payload
        .get("batch_lane_counts")
        .and_then(Value::as_object)
        .map(|rows| {
            rows.iter()
                .map(|(lane, count)| {
                    json!({
                        "lane": clean_text(lane, 60),
                        "count": i64_from_value(Some(count), 0)
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let lane_weights = json!({
        "critical": 1.0,
        "telemetry": 0.8,
        "standard": 0.6,
        "background": 0.3
    });
    let max_batch_size = i64_from_value(attention_contract.get("max_batch_size"), 64).max(1);
    let lane_caps = json!({
        "critical": max_batch_size,
        "telemetry": (max_batch_size / 2).max(1),
        "standard": (max_batch_size / 2).max(1),
        "background": (max_batch_size / 4).max(1)
    });
    let priority_preempt = queue_depth >= RUNTIME_SYNC_WARN_DEPTH
        || pressure_level == "high"
        || pressure_level == "critical";
    let cockpit_receipt_hash = clean_text(
        cockpit_payload
            .get("receipt_hash")
            .and_then(Value::as_str)
            .unwrap_or(""),
        160,
    );
    let attention_status_receipt_hash = clean_text(
        attention_status_payload
            .get("receipt_hash")
            .and_then(Value::as_str)
            .unwrap_or(""),
        160,
    );
    let attention_next_receipt_hash = clean_text(
        attention_next_payload
            .get("receipt_hash")
            .and_then(Value::as_str)
            .unwrap_or(""),
        160,
    );
    let cockpit_freshness_stale = stale_block_count > 0 || !cockpit.ok || cockpit_receipt_hash.is_empty();
    let attention_status_freshness_stale = !attention_status.ok || attention_status_receipt_hash.is_empty();
    let attention_next_freshness_stale = !attention_next.ok
        || attention_next_receipt_hash.is_empty()
        || (pressure_level == "critical" && critical_total_count > 0);
    let stale_surface_count = [cockpit_freshness_stale, attention_status_freshness_stale, attention_next_freshness_stale]
        .iter()
        .filter(|row| **row)
        .count() as i64;
    let freshness_row = |source: &str, sequence_raw: &str, age_ms: i64, stale: bool| -> Value {
        let sequence = clean_text(sequence_raw, 160);
        let sequence_missing = sequence.is_empty();
        let source_sequence = if sequence_missing {
            crate::deterministic_receipt_hash(&json!({
                "source": source,
                "age_ms": age_ms.max(0),
                "stale": stale
            }))
        } else {
            sequence.clone()
        };
        json!({
            "source": source,
            "sequence": if sequence_missing { Value::Null } else { Value::String(sequence) },
            "source_sequence": source_sequence,
            "age_ms": age_ms.max(0),
            "age_seconds": (age_ms.max(0) / 1000),
            "stale": stale || sequence_missing
        })
    };
    let cockpit_freshness = freshness_row(
        "cockpit_receipt",
        cockpit_receipt_hash.as_str(),
        duration_p95.max(duration_avg),
        cockpit_freshness_stale,
    );
    let attention_status_freshness = freshness_row(
        "attention_status_receipt",
        attention_status_receipt_hash.as_str(),
        i64_from_value(attention_status_payload.get("queue_latency_ms"), 0),
        attention_status_freshness_stale,
    );
    let attention_next_freshness = freshness_row(
        "attention_next_receipt",
        attention_next_receipt_hash.as_str(),
        i64_from_value(attention_next_payload.get("wait_ms"), 0),
        attention_next_freshness_stale,
    );

    let mut out = json!({
        "ok": cockpit.ok && attention_status.ok && attention_next.ok,
        "type": "infring_dashboard_runtime_sync",
        "ts": now_iso(),
        "metadata": {
            "team": team,
            "authority": "rust_core_runtime_sync",
            "lanes": {
                "cockpit": cockpit.argv.join(" "),
                "attention_status": attention_status.argv.join(" "),
                "attention_next": attention_next.argv.join(" ")
            }
        },
        "team": team,
        "cockpit_ok": cockpit.ok,
        "attention_status_ok": attention_status.ok,
        "attention_next_ok": attention_next.ok,
        "lanes": {
            "cockpit": cockpit.argv.join(" "),
            "attention_status": attention_status.argv.join(" "),
            "attention_next": attention_next.argv.join(" ")
        },
        "cockpit": {
            "blocks": blocks,
            "block_count": active_block_count,
            "active_block_count": active_block_count,
            "total_block_count": total_block_count,
            "trend": trend,
            "metrics": {
                "duration_ms": {
                    "avg": duration_avg,
                    "p95": duration_p95,
                    "max": duration_max
                },
                "status_counts": status_counts_json,
                "lane_counts": lane_counts_json,
                "slowest_blocks": slowest_blocks,
                "conduit_signals": conduit_signals,
                "conduit_signals_active": conduit_signals,
                "conduit_channels_observed": conduit_channels_observed,
                "conduit_signals_total": conduit_channels_total,
                "stale_block_count": stale_block_count,
                "stale_block_raw_count": stale_block_raw_count,
                "stale_block_dormant_count": stale_block_dormant_count,
                "stale_lanes_top": stale_lanes_top,
                "stale_lanes_dormant_top": stale_lanes_dormant_top,
                "stale_block_threshold_ms": stale_threshold_ms,
                "active_block_count": active_block_count,
                "total_block_count": total_block_count
            },
            "payload_type": cockpit_payload.get("type").cloned().unwrap_or(Value::Null),
            "receipt_hash": cockpit_payload.get("receipt_hash").cloned().unwrap_or(Value::Null)
        },
        "attention_queue": {
            "queue_depth": queue_depth,
            "events": events,
            "critical_visible_count": critical_visible_count,
            "critical_total_count": critical_total_count,
            "critical_events": critical_events,
            "critical_events_full": critical_events_full,
            "telemetry_events": telemetry_events,
            "standard_events": standard_events,
            "background_events": background_events,
            "telemetry_micro_batches": telemetry_micro_batches,
            "lane_weights": lane_weights.clone(),
            "priority_counts": {
                "critical": critical_total_count,
                "telemetry": telemetry_total_count,
                "standard": standard_total_count,
                "background": background_total_count,
                "total": critical_total_count + telemetry_total_count + standard_total_count + background_total_count
            },
            "lane_counts": {
                "critical": critical_total_count,
                "telemetry": telemetry_total_count,
                "standard": standard_total_count,
                "background": background_total_count
            },
            "backpressure": {
                "level": pressure_level,
                "sync_mode": sync_mode,
                "max_queue_depth": max_queue_depth,
                "queue_utilization": queue_utilization,
                "soft_watermark": backpressure_soft_watermark,
                "hard_watermark": backpressure_hard_watermark,
                "cockpit_to_conduit_ratio": cockpit_to_conduit_ratio,
                "conduit_signals": conduit_signals,
                "conduit_signals_raw": conduit_channels_total,
                "conduit_channels_total": conduit_channels_total,
                "conduit_channels_observed": conduit_channels_observed,
                "target_conduit_signals": target_conduit_signals,
                "scale_required": conduit_scale_required,
                "lane_weights": lane_weights.clone(),
                "lane_caps": lane_caps.clone(),
                "priority_preempt": priority_preempt
            },
            "latest": attention_status_payload.get("latest").cloned().unwrap_or(Value::Null),
            "status_type": attention_status_payload.get("type").cloned().unwrap_or(Value::Null),
            "next_type": attention_next_payload.get("type").cloned().unwrap_or(Value::Null),
            "receipt_hashes": {
                "status": attention_status_payload.get("receipt_hash").cloned().unwrap_or(Value::Null),
                "next": attention_next_payload.get("receipt_hash").cloned().unwrap_or(Value::Null)
            }
        },
        "summary": {
            "queue_depth": queue_depth,
            "cockpit_blocks": active_block_count,
            "cockpit_total_blocks": total_block_count,
            "cockpit_stale_blocks": stale_block_count,
            "conduit_signals": conduit_signals,
            "conduit_channels_observed": conduit_channels_observed,
            "conduit_channels_total": conduit_channels_total,
            "target_conduit_signals": target_conduit_signals,
            "conduit_scale_required": conduit_scale_required,
            "attention_batch_count": critical_visible_count
                + telemetry_count
                + standard_count
                + background_count,
            "critical_attention_total": critical_total_count,
            "conduit_signals_raw": conduit_channels_total,
            "sync_mode": sync_mode,
            "backpressure_level": pressure_level,
            "freshness_stale_surfaces": stale_surface_count,
            "freshness_stale": stale_surface_count > 0
        },
        "freshness": {
            "cockpit": cockpit_freshness,
            "attention_status": attention_status_freshness,
            "attention_next": attention_next_freshness,
            "summary": {
                "surface_count": 3,
                "stale_surfaces": stale_surface_count,
                "stale": stale_surface_count > 0
            }
        }
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
