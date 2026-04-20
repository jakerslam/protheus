
fn today_utc() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

fn sha16(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    hex::encode(digest)[..16].to_string()
}

#[derive(Clone, Debug)]
struct Quote {
    symbol: String,
    short_name: String,
    price: f64,
    change: f64,
    change_percent: f64,
    volume: i64,
}

fn quote_from_object(obj: &Map<String, Value>) -> Option<Quote> {
    let symbol = clean_text(obj.get("symbol").and_then(Value::as_str), 32).to_uppercase();
    let price = obj
        .get("regularMarketPrice")
        .and_then(Value::as_f64)
        .or_else(|| obj.get("price").and_then(Value::as_f64))
        .unwrap_or(0.0);
    if symbol.is_empty() || !(price.is_finite() && price > 0.0) {
        return None;
    }
    let short_name = clean_text(
        obj.get("shortName")
            .and_then(Value::as_str)
            .or_else(|| obj.get("longName").and_then(Value::as_str))
            .or_else(|| obj.get("name").and_then(Value::as_str))
            .or_else(|| obj.get("symbol").and_then(Value::as_str)),
        160,
    );
    let change = obj
        .get("regularMarketChange")
        .and_then(Value::as_f64)
        .or_else(|| obj.get("change").and_then(Value::as_f64))
        .unwrap_or(0.0);
    let change_percent = obj
        .get("regularMarketChangePercent")
        .and_then(Value::as_f64)
        .or_else(|| obj.get("changePercent").and_then(Value::as_f64))
        .or_else(|| obj.get("change_percent").and_then(Value::as_f64))
        .unwrap_or(0.0);
    let volume = obj
        .get("regularMarketVolume")
        .and_then(Value::as_i64)
        .or_else(|| obj.get("volume").and_then(Value::as_i64))
        .unwrap_or(0);
    Some(Quote {
        symbol,
        short_name: if short_name.is_empty() {
            "Unknown".to_string()
        } else {
            short_name
        },
        price,
        change,
        change_percent,
        volume,
    })
}

fn walk_quotes(value: &Value, out: &mut BTreeMap<String, Quote>, depth: usize) {
    if depth > 16 {
        return;
    }
    match value {
        Value::Object(obj) => {
            if let Some(quote) = quote_from_object(obj) {
                out.entry(quote.symbol.clone()).or_insert(quote);
            }
            for child in obj.values() {
                walk_quotes(child, out, depth + 1);
            }
        }
        Value::Array(rows) => {
            for row in rows {
                walk_quotes(row, out, depth + 1);
            }
        }
        _ => {}
    }
}

fn extract_quotes_from_html(html: &str) -> Vec<Quote> {
    let patterns = [
        r#"(?s)root\.App\.main\s*=\s*(\{.*?\});"#,
        r#"(?s)window\._initialState\s*=\s*(\{.*?\});"#,
        r#"(?s)"marketSummaryAndSparkResponse":(\{.*?\}),"#,
    ];

    let mut quotes = BTreeMap::<String, Quote>::new();
    for pat in patterns {
        let re = match Regex::new(pat) {
            Ok(v) => v,
            Err(_) => continue,
        };
        for caps in re.captures_iter(html) {
            let raw = caps.get(1).map(|m| m.as_str()).unwrap_or("").trim();
            if raw.is_empty() {
                continue;
            }
            if let Ok(parsed) = serde_json::from_str::<Value>(raw) {
                walk_quotes(&parsed, &mut quotes, 0);
            }
        }
    }
    quotes.into_values().collect::<Vec<_>>()
}

fn quote_to_value(q: &Quote) -> Value {
    json!({
        "symbol": q.symbol,
        "shortName": q.short_name,
        "price": q.price,
        "change": q.change,
        "changePercent": q.change_percent,
        "volume": q.volume
    })
}

fn date_seed(payload: &Map<String, Value>) -> String {
    let raw = clean_text(payload.get("date").and_then(Value::as_str), 32);
    if raw.is_empty() {
        today_utc()
    } else {
        raw
    }
}

fn normalize_seen_ids(payload: &Map<String, Value>) -> Vec<String> {
    payload
        .get("seen_ids")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|v| clean_text(Some(v), 120))
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn format_signed_2(value: f64) -> String {
    if value >= 0.0 {
        format!("+{value:.2}")
    } else {
        format!("{value:.2}")
    }
}
