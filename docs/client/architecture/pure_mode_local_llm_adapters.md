# Pure Mode Local LLM Adapter Spec (llama.cpp + ollama)

## Goal

Provide first-class local model routing in pure mode without Node/TS runtime dependency.

## Adapter Contract

### Common fields
- `adapter_id`
- `provider` (`llama_cpp` | `ollama`)
- `model`
- `context_window`
- `max_tokens`
- `temperature`
- `supports_vision`
- `supports_tool_calls`

### llama.cpp adapter
- transport: local HTTP or direct FFI bridge
- required: model path, quantization metadata, context settings
- health probe: load + token generation smoke check

### ollama adapter
- transport: local HTTP (`/api/generate`, `/api/chat`)
- required: running daemon endpoint + model availability check
- health probe: model pull status + short completion

## Routing Rules

- Pure mode `auto` router can choose only providers marked `pure_compatible=true`.
- Fail closed if no compliant local provider is healthy.
- Every route decision emits deterministic receipt with selection reason.

## CLI

- `infring adapters local status`
- `infring adapters local test --provider=ollama --model=...`
- `infring adapters local test --provider=llama_cpp --model=...`
- `infring route auto --pure`
