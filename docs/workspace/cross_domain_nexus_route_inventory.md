# Cross-Domain Nexus Route Inventory

Status: Canonical architecture inventory
Owner: Jay
Scope: Core, Orchestration Surface, Shell, Gateway, Apps, Packages, and external-agent/plugin boundaries
Effective: April 2026

## Purpose

This inventory makes the current cross-domain Nexus routes explicit enough to audit.

It answers the practical question left after the Layered Nexus Federation resolution: which cross-domain route classes currently exist, where do they enter, where do they exit, what Conduit carries them, what posture protects them, and what proof hooks keep them governable?

## Core Rule

Every cross-domain route row must declare:

- route id;
- route class;
- source domain;
- target domain;
- source Nexus checkpoint;
- target Nexus checkpoint;
- Conduit path;
- Conduit/Scrambler posture;
- lease or capability requirement;
- lifecycle gate;
- receipt requirement;
- owner of truth;
- whether the route is a default projection or a detail route.

If a route cannot name those fields, it is not inventory-compliant.

## Inventory Contract

The machine-readable inventory lives at `client/runtime/config/cross_domain_nexus_route_inventory.json`.

The inventory is intentionally a route contract, not a payload mirror. It records route metadata and checkpoint evidence only. It must not include raw payloads, trace bodies, plan graphs, tool results, policy decisions, authorization state, or runtime state snapshots.

## Required Domain Coverage

The inventory must cover at least one active route involving each of these domains:

- `kernel_core`
- `orchestration_surface`
- `shell`
- `gateway`
- `app`
- `package_sdk`
- `external_agent_or_plugin`
- `cli_gateway_operator`

## Required Route Class Coverage

The inventory must cover the user/system boundary, Shell projection boundary, detail boundary, search boundary, control-plane-to-Core boundary, external agent/plugin ingress boundary, and emergency recovery boundary.

Required route classes are:

- `request_ingress`
- `event_output_egress`
- `health_status`
- `detail_fetch`
- `bounded_search_query`
- `control_plane_coordination`
- `kernel_authority`
- `detail_fetch_sensitive`
- `external_agent_or_plugin_ingress`
- `emergency_recovery`

## Relationship To Existing Policies

This inventory does not replace the Nexus-Conduit-Checkpoint policy. It is the concrete route list that policy requires.

This inventory does not replace the Gateway Ingress/Egress Interface policy. Gateway route classes still define allowed payload fields and default response responsibilities.

This inventory does not replace the Interface Payload Budget policy. Default projection routes must still declare endpoint budgets.

This inventory does not replace the Conduit/Scrambler Posture policy. Sensitive routes must still declare `strong_scrambler` and must not silently downgrade.

## Current Inventory Summary

The initial inventory covers:

- Shell request ingress through Gateway contracts.
- Gateway health/status projection back to Shell.
- Gateway chat-message-window projection back to Shell.
- Shell detail fetch by reference.
- Shell bounded search/query routes.
- Orchestration contract routing into Core Conduit.
- Tool routing authority into Core web Conduit.
- Eval feedback routing into Core eval plane.
- Sensitive Gateway detail fetch into Core.
- External agent/plugin agency ingress into Core.
- Operator daemon-control emergency recovery into Core.
- App to package SDK request ingress.
- Package SDK to Gateway request ingress.

## Enforcement

Enforcement command: `npm run -s ops:nexus:route-inventory:guard`.

Nexus governance command: `npm run -s ops:nexus:governance`.

Architecture policy governance command: `npm run -s ops:policy-refinement:governance`.

The guard validates the structured inventory, checks that checkpoint and Conduit paths exist, verifies required domain and route-class coverage, cross-checks the Conduit/Scrambler required sensitive routes, and fails closed on missing lease/capability, lifecycle, receipt, Nexus checkpoint, or owner-of-truth declarations.
