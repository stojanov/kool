use std::sync::Arc;

use crate::event;

type EventType = Arc<event::Event>;
type ListenerFunc = dyn Fn(EventType) + 'static + Send;
type Listener = Box<ListenerFunc>;

pub struct Signal {
    listeners: Vec<Listener>
}

impl Signal {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new()
        }
    }

    pub fn notify(&self, e: event::Event) {
        let e_ptr = Arc::new(e);

        for f in self.listeners.iter() {
            f(Arc::clone(&e_ptr));
        }
    }

    pub fn connect<F>(&mut self, f: F) where F: Fn(EventType) + 'static + Send {
        self.listeners.push(Box::new(f));
    }
}

