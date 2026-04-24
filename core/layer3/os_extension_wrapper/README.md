# os_extension_wrapper (Layer 3)

Full OS extension wrapper contracts that sit above Layer 2 deterministic scheduling.

- Input: `OsExtensionDescriptor`
- Output: `OsExtensionEnvelope`
- Scope: syscall/driver/namespace extension surfaces with no Layer 0 bypass

This crate expresses extension shape only; authority remains below Layer 3.

It also carries the minimal Layer 3 execution-unit model:

- `ExecutionUnit`
- `ExecutionUnitBudget`
- `ExecutionUnitState`
- `ExecutionUnitTracker`

The tracker records lifecycle and receipt facts only. Layer 2 remains
authoritative for scheduling, admission, queues, and execution lanes.
