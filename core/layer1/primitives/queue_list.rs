// SPDX-License-Identifier: Apache-2.0
// Plane ownership: Layer 1 direct primitives.

use chrono::{DateTime, Utc};
use protheus_nexus_core_v1::{ProvenanceError, ReceiptDraft, ReceiptEmitter, ReceiptSink};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

pub type QueueItemId = Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueueStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct QueueItem {
    pub id: QueueItemId,
    pub payload: Value,
    pub priority: u32,
    pub status: QueueStatus,
    pub created_at: DateTime<Utc>,
    pub last_receipt_id: Option<Uuid>,
}

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("queue item not found: {0}")]
    ItemNotFound(QueueItemId),
    #[error("invalid status transition from {from:?} to {to:?}")]
    InvalidTransition { from: QueueStatus, to: QueueStatus },
    #[error("reprioritize requires pending item: {0}")]
    ReprioritizeNonPending(QueueItemId),
    #[error(transparent)]
    Provenance(#[from] ProvenanceError),
}

#[derive(Clone, Debug)]
struct QueueNode {
    item: QueueItem,
    insertion_seq: u64,
}

#[derive(Clone, Debug, Default)]
pub struct QueueList {
    nodes: Vec<QueueNode>,
    next_seq: u64,
}

impl QueueList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue<S: ReceiptSink>(
        &mut self,
        payload: Value,
        priority: u32,
        emitter: &mut ReceiptEmitter<S>,
    ) -> Result<QueueItem, QueueError> {
        let mut item = QueueItem {
            id: Uuid::new_v4(),
            payload,
            priority,
            status: QueueStatus::Pending,
            created_at: Utc::now(),
            last_receipt_id: None,
        };
        let receipt = emitter.emit(ReceiptDraft {
            parent_id: None,
            op_type: "queue_enqueue",
            subject: Some(item.id.to_string()),
            payload: &json!({"priority": item.priority, "status": "Pending"}),
            actor: "queue_list",
            confidence: None,
        })?;
        item.last_receipt_id = Some(receipt.id);
        self.nodes.push(QueueNode {
            item: item.clone(),
            insertion_seq: self.next_seq,
        });
        self.next_seq = self.next_seq.saturating_add(1);
        Ok(item)
    }

    pub fn peek(&self) -> Option<&QueueItem> {
        self.best_pending_index()
            .map(|index| &self.nodes[index].item)
    }

    pub fn dequeue<S: ReceiptSink>(
        &mut self,
        emitter: &mut ReceiptEmitter<S>,
    ) -> Result<Option<QueueItem>, QueueError> {
        let Some(index) = self.best_pending_index() else {
            return Ok(None);
        };
        let node = &mut self.nodes[index];
        let receipt = emitter.emit(ReceiptDraft {
            parent_id: node.item.last_receipt_id,
            op_type: "queue_dequeue",
            subject: Some(node.item.id.to_string()),
            payload: &json!({"from":"Pending","to":"InProgress"}),
            actor: "queue_list",
            confidence: None,
        })?;
        node.item.status = QueueStatus::InProgress;
        node.item.last_receipt_id = Some(receipt.id);
        Ok(Some(node.item.clone()))
    }

    pub fn mark_status<S: ReceiptSink>(
        &mut self,
        id: QueueItemId,
        status: QueueStatus,
        emitter: &mut ReceiptEmitter<S>,
    ) -> Result<(), QueueError> {
        let node = self
            .nodes
            .iter_mut()
            .find(|row| row.item.id == id)
            .ok_or(QueueError::ItemNotFound(id))?;
        if node.item.status == status {
            return Ok(());
        }
        if !is_valid_transition(&node.item.status, &status) {
            return Err(QueueError::InvalidTransition {
                from: node.item.status.clone(),
                to: status,
            });
        }
        let next_status = status.clone();
        let receipt = emitter.emit(ReceiptDraft {
            parent_id: node.item.last_receipt_id,
            op_type: "queue_mark_status",
            subject: Some(node.item.id.to_string()),
            payload: &json!({"from": format!("{:?}", node.item.status), "to": format!("{:?}", next_status)}),
            actor: "queue_list",
            confidence: None,
        })?;
        node.item.status = status;
        node.item.last_receipt_id = Some(receipt.id);
        Ok(())
    }

    pub fn reprioritize<S: ReceiptSink>(
        &mut self,
        id: QueueItemId,
        new_priority: u32,
        emitter: &mut ReceiptEmitter<S>,
    ) -> Result<(), QueueError> {
        let node = self
            .nodes
            .iter_mut()
            .find(|row| row.item.id == id)
            .ok_or(QueueError::ItemNotFound(id))?;
        if node.item.status != QueueStatus::Pending {
            return Err(QueueError::ReprioritizeNonPending(id));
        }
        if node.item.priority == new_priority {
            return Ok(());
        }
        let receipt = emitter.emit(ReceiptDraft {
            parent_id: node.item.last_receipt_id,
            op_type: "queue_reprioritize",
            subject: Some(node.item.id.to_string()),
            payload: &json!({"from": node.item.priority, "to": new_priority}),
            actor: "queue_list",
            confidence: None,
        })?;
        node.item.priority = new_priority;
        node.item.last_receipt_id = Some(receipt.id);
        Ok(())
    }

    fn best_pending_index(&self) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.item.status == QueueStatus::Pending)
            .max_by(|(_, left), (_, right)| {
                left.item
                    .priority
                    .cmp(&right.item.priority)
                    .then_with(|| right.insertion_seq.cmp(&left.insertion_seq))
            })
            .map(|(index, _)| index)
    }
}

fn is_valid_transition(from: &QueueStatus, to: &QueueStatus) -> bool {
    matches!(
        (from, to),
        (QueueStatus::Pending, QueueStatus::InProgress)
            | (QueueStatus::Pending, QueueStatus::Completed)
            | (QueueStatus::Pending, QueueStatus::Failed)
            | (QueueStatus::InProgress, QueueStatus::Completed)
            | (QueueStatus::InProgress, QueueStatus::Failed)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use protheus_nexus_core_v1::{InMemoryReceiptSink, ReceiptEmitter};
    use serde_json::json;

    #[test]
    fn enqueue_orders_by_priority_then_fifo() {
        let mut queue = QueueList::new();
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        let _a = queue
            .enqueue(json!({"task":"a"}), 1, &mut emitter)
            .expect("enqueue a");
        let b = queue
            .enqueue(json!({"task":"b"}), 5, &mut emitter)
            .expect("enqueue b");
        let c = queue
            .enqueue(json!({"task":"c"}), 5, &mut emitter)
            .expect("enqueue c");
        assert_eq!(queue.peek().map(|item| item.id), Some(b.id));
        let first = queue
            .dequeue(&mut emitter)
            .expect("dequeue")
            .expect("first item");
        assert_eq!(first.id, b.id);
        assert_eq!(queue.peek().map(|item| item.id), Some(c.id));
    }

    #[test]
    fn peek_is_read_only() {
        let mut queue = QueueList::new();
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        queue
            .enqueue(json!({"task":"peek"}), 2, &mut emitter)
            .expect("enqueue");
        let before = emitter.sink().receipts.len();
        let item = queue.peek().expect("peek");
        assert_eq!(item.status, QueueStatus::Pending);
        assert_eq!(before, emitter.sink().receipts.len());
    }

    #[test]
    fn dequeue_marks_item_in_progress_without_removing_it() {
        let mut queue = QueueList::new();
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        let item = queue
            .enqueue(json!({"task":"work"}), 10, &mut emitter)
            .expect("enqueue");
        let dequeued = queue.dequeue(&mut emitter).expect("dequeue").expect("item");
        assert_eq!(dequeued.id, item.id);
        let stored = queue
            .nodes
            .iter()
            .find(|row| row.item.id == item.id)
            .expect("stored");
        assert_eq!(stored.item.status, QueueStatus::InProgress);
        assert_eq!(queue.nodes.len(), 1);
    }

    #[test]
    fn valid_status_transitions_succeed() {
        let mut queue = QueueList::new();
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        let item = queue
            .enqueue(json!({"task":"trans"}), 1, &mut emitter)
            .expect("enqueue");
        queue
            .mark_status(item.id, QueueStatus::InProgress, &mut emitter)
            .expect("to in progress");
        queue
            .mark_status(item.id, QueueStatus::Completed, &mut emitter)
            .expect("to completed");
        let stored = queue
            .nodes
            .iter()
            .find(|row| row.item.id == item.id)
            .expect("stored");
        assert_eq!(stored.item.status, QueueStatus::Completed);
    }

    #[test]
    fn invalid_transitions_fail() {
        let mut queue = QueueList::new();
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        let item = queue
            .enqueue(json!({"task":"trans"}), 1, &mut emitter)
            .expect("enqueue");
        queue
            .mark_status(item.id, QueueStatus::Completed, &mut emitter)
            .expect("complete");
        let err = queue
            .mark_status(item.id, QueueStatus::InProgress, &mut emitter)
            .expect_err("completed to inprogress should fail");
        assert!(matches!(err, QueueError::InvalidTransition { .. }));
    }

    #[test]
    fn same_status_mark_is_idempotent_no_op_without_new_receipt() {
        let mut queue = QueueList::new();
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        let item = queue
            .enqueue(json!({"task":"noop"}), 1, &mut emitter)
            .expect("enqueue");
        queue
            .mark_status(item.id, QueueStatus::Completed, &mut emitter)
            .expect("complete");
        let before = emitter.sink().receipts.len();
        queue
            .mark_status(item.id, QueueStatus::Completed, &mut emitter)
            .expect("idempotent");
        assert_eq!(before, emitter.sink().receipts.len());
    }

    #[test]
    fn reprioritize_only_works_for_pending_items() {
        let mut queue = QueueList::new();
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        let pending = queue
            .enqueue(json!({"task":"pending"}), 1, &mut emitter)
            .expect("enqueue pending");
        queue
            .reprioritize(pending.id, 9, &mut emitter)
            .expect("reprioritize pending");
        queue
            .dequeue(&mut emitter)
            .expect("dequeue")
            .expect("dequeued");
        let err = queue
            .reprioritize(pending.id, 10, &mut emitter)
            .expect_err("reprioritize non-pending should fail");
        assert!(matches!(err, QueueError::ReprioritizeNonPending(_)));
    }
}
