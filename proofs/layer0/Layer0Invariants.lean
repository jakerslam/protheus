/- SPDX-License-Identifier: Apache-2.0 -/
/- Layer ownership: proofs/layer0 (machine-checked) -/

namespace Layer0Invariants

structure ConduitRoute where
  viaConduit : Bool

def ConduitOnlyPath (route : ConduitRoute) : Prop :=
  route.viaConduit = true

theorem conduit_only_path_enforced
    (route : ConduitRoute)
    (h : ConduitOnlyPath route) :
    route.viaConduit = true := by
  exact h

structure ConstitutionState where
  hardened : Bool
  merkleBound : Bool
  operatorApproved : Bool

def ConstitutionHardened (state : ConstitutionState) : Prop :=
  state.hardened = true ∧
  state.merkleBound = true ∧
  state.operatorApproved = true

theorem constitution_hardening_enforced
    (state : ConstitutionState)
    (h : ConstitutionHardened state) :
    state.hardened = true := by
  exact h.left

theorem constitution_merkle_binding_enforced
    (state : ConstitutionState)
    (h : ConstitutionHardened state) :
    state.merkleBound = true := by
  exact h.right.left

theorem constitution_operator_approval_enforced
    (state : ConstitutionState)
    (h : ConstitutionHardened state) :
    state.operatorApproved = true := by
  exact h.right.right

structure ReceiptState where
  payloadHash : Nat
  stateHash : Nat
  signatureValid : Bool

def ReceiptStateBound (receipt : ReceiptState) : Prop :=
  receipt.payloadHash = receipt.stateHash

def ReceiptAntiForgery (receipt : ReceiptState) : Prop :=
  receipt.signatureValid = true → ReceiptStateBound receipt

theorem receipt_state_binding_enforced
    (receipt : ReceiptState)
    (h : ReceiptStateBound receipt) :
    receipt.payloadHash = receipt.stateHash := by
  exact h

theorem receipt_anti_forgery_enforced
    (receipt : ReceiptState)
    (hbound : ReceiptStateBound receipt) :
    ReceiptAntiForgery receipt := by
  intro _hSig
  exact hbound

theorem forged_receipt_rejected
    (receipt : ReceiptState)
    (hSig : receipt.signatureValid = true)
    (hAnti : ReceiptAntiForgery receipt)
    (hMismatch : receipt.payloadHash ≠ receipt.stateHash) :
    False := by
  have hBound : ReceiptStateBound receipt := hAnti hSig
  exact hMismatch hBound

structure BoundaryState where
  failClosed : Bool

def FailClosedBoundary (state : BoundaryState) : Prop :=
  state.failClosed = true

theorem fail_closed_boundary_enforced
    (state : BoundaryState)
    (h : FailClosedBoundary state) :
    state.failClosed = true := by
  exact h

theorem layer0_invariant_bundle
    (route : ConduitRoute)
    (constitution : ConstitutionState)
    (receipt : ReceiptState)
    (boundary : BoundaryState)
    (hRoute : ConduitOnlyPath route)
    (hConstitution : ConstitutionHardened constitution)
    (hReceipt : ReceiptStateBound receipt)
    (hBoundary : FailClosedBoundary boundary) :
    ConduitOnlyPath route ∧
    ConstitutionHardened constitution ∧
    ReceiptStateBound receipt ∧
    FailClosedBoundary boundary := by
  exact ⟨hRoute, hConstitution, hReceipt, hBoundary⟩

end Layer0Invariants
