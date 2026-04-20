
fn map_quotes(payload: &Map<String, Value>) -> Value {

    let max_items = clamp_u64(payload, "max_items", 20, 1, 200) as usize;
    let source = clean_text(payload.get("source").and_then(Value::as_str), 64);
    let source = if source.is_empty() {
        "stock_market".to_string()
    } else {
        source
    };
    let date = date_seed(payload);
    let collected_at = now_iso();
    let mut seen = normalize_seen_ids(payload)
        .into_iter()
        .collect::<HashSet<_>>();

    let rows = payload
        .get("quotes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut items = Vec::<Value>::new();
    for row in rows {
        if items.len() >= max_items {
            break;
        }
        let quote = row.as_object().and_then(quote_from_object).or_else(|| {
            // Accept canonicalized quote shape from extract-quotes.
            row.as_object().and_then(|obj| {
                let symbol =
                    clean_text(obj.get("symbol").and_then(Value::as_str), 32).to_uppercase();
                let price = obj.get("price").and_then(Value::as_f64).unwrap_or(0.0);
                if symbol.is_empty() || !(price.is_finite() && price > 0.0) {
                    return None;
                }
                Some(Quote {
                    symbol,
                    short_name: clean_text(obj.get("shortName").and_then(Value::as_str), 160),
                    price,
                    change: obj.get("change").and_then(Value::as_f64).unwrap_or(0.0),
                    change_percent: obj
                        .get("changePercent")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0),
                    volume: obj.get("volume").and_then(Value::as_i64).unwrap_or(0),
                })
            })
        });
        let q = match quote {
            Some(v) => v,
            None => continue,
        };

        let id = sha16(&format!("stock-{}-{}-{:.4}", q.symbol, date, q.price));
        if seen.contains(&id) {
            continue;
        }
        seen.insert(id.clone());

        let is_index = q.symbol.starts_with('^');
        let signal = q.change_percent.abs() > 2.0 || q.volume > 10_000_000;
        let signal_type = if is_index { "index" } else { "equity" };
        let movement_tag = if q.change > 0.0 {
            "gainer"
        } else if q.change < 0.0 {
            "loser"
        } else {
            "unchanged"
        };

        items.push(json!({
            "id": id,
            "collected_at": collected_at,
            "url": quote_url(&q.symbol),
            "title": format!(
                "{}: ${:.2} ({}, {}%)",
                if q.short_name.is_empty() { q.symbol.clone() } else { q.short_name.clone() },
                q.price,
                format_signed_2(q.change),
                format_signed_2(q.change_percent)
            ),
            "description": format!("Volume: {}. Market data for {}.", q.volume, q.symbol),
            "symbol": q.symbol,
            "price": q.price,
            "change": q.change,
            "change_percent": q.change_percent,
            "volume": q.volume,
            "signal_type": signal_type,
            "signal": signal,
            "source": source,
            "tags": ["finance", "market", movement_tag],
            "topics": ["finance", "market"],
            "bytes": 0
        }));
    }

    let mut seen_ids = seen.into_iter().collect::<Vec<_>>();
    seen_ids.sort();
    if seen_ids.len() > 2000 {
        let drop = seen_ids.len() - 2000;
        seen_ids.drain(0..drop);
    }

    json!({
        "ok": true,
        "items": items,
        "seen_ids": seen_ids
    })
}

fn fallback_indices(payload: &Map<String, Value>) -> Value {
    let indices = [
        ("^GSPC", "S&P 500", "index"),
        ("^IXIC", "NASDAQ Composite", "index"),
        ("^DJI", "Dow Jones Industrial Average", "index"),
        ("^RUT", "Russell 2000", "index"),
        ("^VIX", "CBOE Volatility Index", "volatility"),
    ];
    let max_items = clamp_u64(payload, "max_items", 20, 1, 200) as usize;
    let date = date_seed(payload);
    let collected_at = now_iso();
    let mut seen = normalize_seen_ids(payload)
        .into_iter()
        .collect::<HashSet<_>>();

    let mut items = Vec::<Value>::new();
    for (symbol, name, signal_type) in indices {
        if items.len() >= max_items {
            break;
        }
        let id = sha16(&format!("stock-{symbol}-{date}"));
        if seen.contains(&id) {
            continue;
        }
        seen.insert(id.clone());
        items.push(json!({
            "id": id,
            "collected_at": collected_at,
            "url": quote_url(symbol),
            "title": format!("{name} - Market Index"),
            "description": "Major market index tracking. Monitor for significant moves.",
            "symbol": symbol,
            "signal_type": signal_type,
            "signal": true,
            "source": "stock_market",
            "tags": ["finance", "index", "market", "fallback"],
            "topics": ["finance", "market"],
            "bytes": 0
        }));
    }

    let mut seen_ids = seen.into_iter().collect::<Vec<_>>();
    seen_ids.sort();
    if seen_ids.len() > 2000 {
        let drop = seen_ids.len() - 2000;
        seen_ids.drain(0..drop);
    }

    json!({
        "ok": true,
        "items": items,
        "seen_ids": seen_ids
    })
}

fn quote_url(symbol: &str) -> String {
    format!("https://finance.yahoo.com/quote/{}", urlencoding::encode(symbol))
}

fn command_prepare_run(root: &Path, payload: &Map<String, Value>) -> Value {
    let force = as_bool(payload.get("force"), false);
    let min_hours = as_f64(payload.get("min_hours"), 1.0).clamp(0.0, 24.0 * 365.0);
    let meta_path = meta_path_for(root, payload);
    let meta = normalize_meta_value(Some(&read_json(&meta_path, normalize_meta_value(None))));
    let last_run_ms = meta
        .get("last_run")
        .and_then(Value::as_str)
        .and_then(parse_iso_ms);
    let hours_since_last =
        last_run_ms.map(|ms| ((Utc::now().timestamp_millis() - ms) as f64 / 3_600_000.0).max(0.0));
    let skipped = !force && hours_since_last.map(|h| h < min_hours).unwrap_or(false);
    json!({
        "ok": true,
        "collector_id": COLLECTOR_ID,
        "force": force,
        "min_hours": min_hours,
        "hours_since_last": hours_since_last,
        "skipped": skipped,
        "reason": if skipped { Value::String("cadence".to_string()) } else { Value::Null },
        "meta": meta,
        "meta_path": meta_path.display().to_string()
    })
}

fn command_build_fetch_plan(_payload: &Map<String, Value>) -> Value {
    json!({
        "ok": true,
        "collector_id": COLLECTOR_ID,
        "requests": [
            {
                "key": "market_html",
                "url": "https://finance.yahoo.com/markets/",
                "required": true,
                "accept": "application/json,text/html,*/*"
            }
        ]
    })
}
