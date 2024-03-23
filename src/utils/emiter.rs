use std::collections::HashMap;


type EventCallback<T> = Box<dyn Fn(&T)>;

pub struct EventManager<T> {
    subscribers: HashMap<String, Vec<EventCallback<T>>>,
}

impl<T> EventManager<T> {
    pub fn new() -> EventManager<T> {
        EventManager {
            subscribers: HashMap::new(),
        }
    }

    pub fn subscribe(&mut self, event_name: &str, callback: EventCallback<T>) {
        let subscribers = self.subscribers.entry(event_name.to_string()).or_insert(vec![]);
        subscribers.push(callback);
    }

    pub fn publish(&self, event_name: &str, event: &T) {
        if let Some(subscribers) = self.subscribers.get(event_name) {
            for callback in subscribers {
                callback(event);
            }
        }
    }
}


unsafe impl<T> Send for EventManager<T> {}
unsafe impl<T> Sync for EventManager<T> {}
