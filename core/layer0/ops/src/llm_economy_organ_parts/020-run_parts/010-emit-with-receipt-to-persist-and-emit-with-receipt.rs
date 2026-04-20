fn emit_with_receipt(out: &mut Value) {
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(out));
    print_receipt(out);
}

fn persist_and_emit_with_receipt(latest: &Path, history: &Path, out: &mut Value) {
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(out));
    write_json(latest, out);
    append_jsonl(history, out);
    print_receipt(out);
}
