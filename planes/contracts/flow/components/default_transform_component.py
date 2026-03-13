def run(payload):
    if isinstance(payload, dict):
        data = payload.copy()
    else:
        data = {"value": payload}
    data["transformed"] = True
    return {"ok": True, "payload": data}
