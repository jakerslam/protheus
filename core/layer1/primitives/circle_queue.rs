// SPDX-License-Identifier: Apache-2.0
// Plane ownership: Layer 1 direct primitives.

use protheus_nexus_core_v1::{ProvenanceError, ReceiptDraft, ReceiptEmitter, ReceiptSink};
use serde::Serialize;
use serde_json::json;
use std::collections::VecDeque;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IterOrder {
    OldestToNewest,
    NewestToOldest,
}

#[derive(Debug, Error)]
pub enum CircleQueueError {
    #[error("capacity must be greater than zero")]
    InvalidCapacity,
    #[error(transparent)]
    Provenance(#[from] ProvenanceError),
}

#[derive(Clone, Debug)]
pub struct CircleQueue<T> {
    cap: usize,
    buf: VecDeque<T>,
}

impl<T> CircleQueue<T> {
    pub fn with_capacity(cap: usize) -> Result<Self, CircleQueueError> {
        if cap == 0 {
            return Err(CircleQueueError::InvalidCapacity);
        }
        Ok(Self {
            cap,
            buf: VecDeque::with_capacity(cap),
        })
    }

    pub fn peek_oldest(&self) -> Option<&T> {
        self.buf.front()
    }

    pub fn peek_newest(&self) -> Option<&T> {
        self.buf.back()
    }

    pub fn iter(&self, order: IterOrder) -> impl Iterator<Item = &T> {
        let refs = match order {
            IterOrder::OldestToNewest => self.buf.iter().collect::<Vec<_>>(),
            IterOrder::NewestToOldest => self.buf.iter().rev().collect::<Vec<_>>(),
        };
        refs.into_iter()
    }
}

impl<T> CircleQueue<T>
where
    T: Serialize + Clone,
{
    pub fn push<S: ReceiptSink>(
        &mut self,
        item: T,
        emitter: &mut ReceiptEmitter<S>,
    ) -> Result<(), CircleQueueError> {
        if self.buf.len() == self.cap {
            let dropped = self.buf.pop_front().expect("capacity > 0 and full");
            let payload = json!({"dropped": dropped, "inserted": item.clone()});
            emitter.emit(ReceiptDraft {
                parent_id: None,
                op_type: "circle_overwrite",
                subject: Some("circle_queue".to_string()),
                payload: &payload,
                actor: "circle_queue",
                confidence: None,
            })?;
        } else {
            let payload = json!({"inserted": item.clone()});
            emitter.emit(ReceiptDraft {
                parent_id: None,
                op_type: "circle_push",
                subject: Some("circle_queue".to_string()),
                payload: &payload,
                actor: "circle_queue",
                confidence: None,
            })?;
        }
        self.buf.push_back(item);
        Ok(())
    }

    pub fn pop_oldest<S: ReceiptSink>(
        &mut self,
        emitter: &mut ReceiptEmitter<S>,
    ) -> Result<Option<T>, CircleQueueError> {
        let Some(item) = self.buf.pop_front() else {
            return Ok(None);
        };
        let payload = json!({"removed": item.clone()});
        emitter.emit(ReceiptDraft {
            parent_id: None,
            op_type: "circle_pop_oldest",
            subject: Some("circle_queue".to_string()),
            payload: &payload,
            actor: "circle_queue",
            confidence: None,
        })?;
        Ok(Some(item))
    }

    pub fn clear<S: ReceiptSink>(
        &mut self,
        emitter: &mut ReceiptEmitter<S>,
    ) -> Result<(), CircleQueueError> {
        if self.buf.is_empty() {
            return Ok(());
        }
        let payload = json!({"count": self.buf.len()});
        emitter.emit(ReceiptDraft {
            parent_id: None,
            op_type: "circle_clear",
            subject: Some("circle_queue".to_string()),
            payload: &payload,
            actor: "circle_queue",
            confidence: None,
        })?;
        self.buf.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protheus_nexus_core_v1::{InMemoryReceiptSink, ReceiptEmitter};
    use serde_json::json;

    #[test]
    fn capacity_zero_is_rejected() {
        let err = CircleQueue::<i32>::with_capacity(0).expect_err("cap=0 should fail");
        assert!(matches!(err, CircleQueueError::InvalidCapacity));
    }

    #[test]
    fn push_fills_buffer_in_order() {
        let mut queue = CircleQueue::with_capacity(3).expect("queue");
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        queue.push(1_i32, &mut emitter).expect("push 1");
        queue.push(2_i32, &mut emitter).expect("push 2");
        queue.push(3_i32, &mut emitter).expect("push 3");
        let values = queue
            .iter(IterOrder::OldestToNewest)
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(values, vec![1, 2, 3]);
    }

    #[test]
    fn full_push_overwrites_oldest() {
        let mut queue = CircleQueue::with_capacity(2).expect("queue");
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        queue.push(1_i32, &mut emitter).expect("push 1");
        queue.push(2_i32, &mut emitter).expect("push 2");
        queue.push(3_i32, &mut emitter).expect("overwrite");
        let values = queue
            .iter(IterOrder::OldestToNewest)
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(values, vec![2, 3]);
        let ops = emitter
            .sink()
            .receipts
            .iter()
            .map(|row| row.op_type.clone())
            .collect::<Vec<_>>();
        assert_eq!(ops, vec!["circle_push", "circle_push", "circle_overwrite"]);
    }

    #[test]
    fn peek_oldest_and_newest_are_correct() {
        let mut queue = CircleQueue::with_capacity(3).expect("queue");
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        queue.push(10_i32, &mut emitter).expect("push");
        queue.push(11_i32, &mut emitter).expect("push");
        assert_eq!(queue.peek_oldest(), Some(&10));
        assert_eq!(queue.peek_newest(), Some(&11));
    }

    #[test]
    fn pop_oldest_returns_oldest() {
        let mut queue = CircleQueue::with_capacity(2).expect("queue");
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        queue.push("a".to_string(), &mut emitter).expect("push a");
        queue.push("b".to_string(), &mut emitter).expect("push b");
        let popped = queue.pop_oldest(&mut emitter).expect("pop").expect("value");
        assert_eq!(popped, "a".to_string());
        let values = queue
            .iter(IterOrder::OldestToNewest)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(values, vec!["b".to_string()]);
    }

    #[test]
    fn iter_supports_both_orders() {
        let mut queue = CircleQueue::with_capacity(3).expect("queue");
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        queue.push(json!({"v":1}), &mut emitter).expect("push 1");
        queue.push(json!({"v":2}), &mut emitter).expect("push 2");
        queue.push(json!({"v":3}), &mut emitter).expect("push 3");
        let old_to_new = queue
            .iter(IterOrder::OldestToNewest)
            .map(|row| row["v"].as_i64().expect("i64"))
            .collect::<Vec<_>>();
        let new_to_old = queue
            .iter(IterOrder::NewestToOldest)
            .map(|row| row["v"].as_i64().expect("i64"))
            .collect::<Vec<_>>();
        assert_eq!(old_to_new, vec![1, 2, 3]);
        assert_eq!(new_to_old, vec![3, 2, 1]);
    }

    #[test]
    fn clear_empties_the_buffer() {
        let mut queue = CircleQueue::with_capacity(2).expect("queue");
        let mut emitter = ReceiptEmitter::new(InMemoryReceiptSink::default());
        queue.push(1_i32, &mut emitter).expect("push");
        queue.push(2_i32, &mut emitter).expect("push");
        queue.clear(&mut emitter).expect("clear");
        assert!(queue.peek_oldest().is_none());
        let before = emitter.sink().receipts.len();
        queue.clear(&mut emitter).expect("clear noop");
        assert_eq!(before, emitter.sink().receipts.len());
    }
}
