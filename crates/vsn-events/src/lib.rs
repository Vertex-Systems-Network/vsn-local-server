use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub topic: String,
    pub timestamp_unix_ms: u128,
    pub payload: Value,
}

#[derive(Clone, Default)]
pub struct EventBus {
    subscribers: Arc<Mutex<Vec<Sender<Event>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn subscribe(&self) -> Receiver<Event> {
        let (tx, rx) = mpsc::channel();
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.push(tx);
        }
        rx
    }
    pub fn publish(&self, event: Event) -> usize {
        let mut delivered = 0;
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.retain(|tx| match tx.send(event.clone()) {
                Ok(()) => {
                    delivered += 1;
                    true
                }
                Err(_) => false,
            });
        }
        delivered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fanout_works() {
        let bus = EventBus::new();
        let a = bus.subscribe();
        let b = bus.subscribe();
        let e = Event {
            topic: "x".into(),
            timestamp_unix_ms: 1,
            payload: Value::Null,
        };
        assert_eq!(bus.publish(e), 2);
        assert!(a.recv().is_ok());
        assert!(b.recv().is_ok());
    }
}
