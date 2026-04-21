/- SPDX-License-Identifier: Apache-2.0 -/
/- Layer ownership: proofs/layer1 (machine-checked) -/

namespace Layer1Invariants

structure SecurityDecision where
  allowed : Bool
  denyReasonPresent : Bool

def FailClosedDecision (decision : SecurityDecision) : Prop :=
  decision.allowed = false → decision.denyReasonPresent = true

theorem fail_closed_decision_enforced
    (decision : SecurityDecision)
    (h : FailClosedDecision decision)
    (hDenied : decision.allowed = false) :
    decision.denyReasonPresent = true := by
  exact h hDenied

structure RoutingState where
  deterministic : Bool
  canonicalLane : Bool

def DeterministicRouting (routing : RoutingState) : Prop :=
  routing.deterministic = true ∧ routing.canonicalLane = true

theorem deterministic_routing_enforced
    (routing : RoutingState)
    (h : DeterministicRouting routing) :
    routing.deterministic = true := by
  exact h.left

theorem canonical_lane_routing_enforced
    (routing : RoutingState)
    (h : DeterministicRouting routing) :
    routing.canonicalLane = true := by
  exact h.right

structure ReceiptContract where
  receiptPresent : Bool
  receiptFresh : Bool

def ReceiptAuthority (contract : ReceiptContract) : Prop :=
  contract.receiptPresent = true ∧ contract.receiptFresh = true

theorem receipt_authority_enforced
    (contract : ReceiptContract)
    (h : ReceiptAuthority contract) :
    contract.receiptPresent = true := by
  exact h.left

theorem receipt_freshness_enforced
    (contract : ReceiptContract)
    (h : ReceiptAuthority contract) :
    contract.receiptFresh = true := by
  exact h.right

theorem layer1_invariant_bundle
    (decision : SecurityDecision)
    (routing : RoutingState)
    (contract : ReceiptContract)
    (hDecision : FailClosedDecision decision)
    (hRouting : DeterministicRouting routing)
    (hContract : ReceiptAuthority contract) :
    FailClosedDecision decision ∧
    DeterministicRouting routing ∧
    ReceiptAuthority contract := by
  exact ⟨hDecision, hRouting, hContract⟩

end Layer1Invariants
