# Substrate Plane

The substrate plane defines where and how workloads run.

## Node Is The Primitive

A node can be a microcontroller, laptop, VM, GPU/NPU host, remote QPU, or future neural-link endpoint.
Each node exports a minimum contract:

- tasks
- objects
- capabilities
- resources
- events
- policy constraints

## Substrate Descriptors

Every node/runtime adapter must declare:

- value domain (binary/ternary/probabilistic/vector/quantum)
- determinism class
- latency class
- energy envelope
- isolation mechanism
- observability guarantees
- privacy locality
- degradation behavior

Schema: `planes/substrate/substrate_descriptor.schema.json`.

## Degradation Contracts

Every substrate must expose explicit fallback behavior, including:

- local model -> remote model
- quantum -> classical approximation
- neural link -> conventional UI
- capability hardware -> software sandbox

Schema: `planes/substrate/degradation_contract.schema.json`.
