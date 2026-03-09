# photo-grit

Historical restoration of the legacy multimodal image/signal sensor lane.

Source provenance (verbatim restore):
- commit: `af8d1241afd1fb4b25c8edbd738329ae26cd8391`
- `systems/sensory/multimodal_signal_adapter_plane.{ts,js}`
- `config/multimodal_signal_adapter_policy.json`
- `memory/tools/tests/multimodal_signal_adapter_plane.test.js`

No behavior rewrites were applied during restoration.

## Local validation

Because this app is restored outside its original root layout, the legacy global
security gate in `lib/ts_bootstrap.js` may require bypass for standalone runs:

```bash
PROTHEUS_SECURITY_GLOBAL_GATE=0 node examples/apps/photo-grit/memory/tools/tests/multimodal_signal_adapter_plane.test.js
```
