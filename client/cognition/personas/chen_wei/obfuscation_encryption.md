# Chen Wei Obfuscation & Encryption Config

- **Enabled:** false
- **Mode:** off
- **Key Env Var:** INFRING_PERSONA_ENCRYPTION_KEY

## Modes

- off: plaintext markdown files (default)
- obfuscate: reversible base64 wrapper for low-friction concealment
- encrypt: AES-256-GCM envelope (requires key env var)

## Scope

When enabled, persona writable surfaces (correspondence/feed/memory) may be stored protected at rest.
