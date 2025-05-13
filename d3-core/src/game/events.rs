use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;

// Define a type alias for event handlers without messages
type EventHandler = Rc<RefCell<dyn FnMut()>>;

#[derive(Clone)]
pub struct EventEmitter {
    events: HashMap<String, Vec<EventHandler>>,
}

impl EventEmitter {
    // Create a new EventEmitter
    pub fn new() -> Self {
        EventEmitter {
            events: HashMap::new(),
        }
    }

    // Subscribe to an event
    pub fn on(&mut self, event_type: &str, handler: EventHandler) {
        self.events
            .entry(event_type.to_string())
            .or_insert(Vec::new())
            .push(handler);
    }

    // Emit an event
    pub fn emit(&mut self, event_type: &str) {
        if let Some(handlers) = self.events.get_mut(event_type) {
            for handler in handlers.iter_mut() {
                (handler.borrow_mut())();
            }
        }
    }
}

// Implement Debug for EventEmitter
impl fmt::Debug for EventEmitter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventEmitter")
            .field("events", &self.events.keys().collect::<Vec<_>>())
            .finish()
    }
}
