# Local RAG App (V6-MEMORY-008 / V6-MEMORY-009)

Layer: app (thin workflow surface).

Purpose:
- one-command local RAG ingestion/search/chat flows
- stable memory-library command surface over core memory runtime

Core integration contract:
- all retrieval/ingestion actions route through core `memory-ambient` and memory runtime
- deterministic receipts are emitted by core
- no app-level authority or policy logic

Quick commands:
- `node apps/local-rag/run.js start`
- `node apps/local-rag/run.js ingest --path=docs`
- `node apps/local-rag/run.js search --q="what changed"`
- `node apps/local-rag/run.js chat --q="summarize receipts"`
- `node apps/local-rag/run.js memory search --q="node id"`
