// authevent module produces authorization events at a given interval for
// the state machine to consume.
use super::Event;
use std::sync::mpsc;
use std::thread;
use std::time;

use super::EventProducer;

pub struct AuthorizationEvent {
    interval: time::Duration,
    publisher: mpsc::Sender<Event>,
    events: mpsc::Receiver<Event>,
}
impl AuthorizationEvent {
    pub fn new() -> AuthorizationEvent {
        let (tx1, rx) = mpsc::channel();
        AuthorizationEvent {
            interval: time::Duration::from_secs(30),
            publisher: tx1,
            events: rx,
        }
    }
    pub fn start(&self) {
        let interval = self.interval.clone();
        let tx1 = mpsc::Sender::clone(&self.publisher);
        thread::spawn(move || {
            tx1.send(Event::AuthorizeAttempt).unwrap(); // Initally send an authorization attempt event
            loop {
                thread::sleep(interval);
                match tx1.send(Event::AuthorizeAttempt) {
                    Ok(_) => println!("Successfully sent Authorization attemp Event"),
                    Err(e) => println!("Failed to send the authorization attempt Event: {}", e),
                }
            }
        });
    }
}

impl EventProducer for AuthorizationEvent {
    // Reads from the receiving end of the event channel,
    // and returns the next scheduled event. If noone are ready, it blocks.
    fn next(&self) -> Event {
        self.events.recv().unwrap()
    }
}
