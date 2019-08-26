use log::{debug, info, trace, warn};
use std::process::Command;
use std::time;

// use rsa::{PublicKey, RSAPrivateKey, PaddingScheme};
// use rand::rngs::OsRng;

mod client;
mod syncevent; // Bring the syncevent module into scope // Bring the client into scope
use client::Client;

trait State {
    fn mutate(&self, context: &Context, client: &Client) -> Box<dyn State>;
    fn name<'a>(&'a self) -> &'a str;
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({})", self.name())
    }
}

// struct AuthorizationManager {

// }

enum ExternalState {
    Init,
    Idle,
    Sync,
    Download,
    ArtifactInstall,
    ArtifactReboot,
    ArtifactCommit,
    ArtifactRollback,
    ArtifactRollbackReboot,
    ArtifactFailure,
}

enum Event {
    AuthorizeAttempt,
    CheckForUpdate,
    SendInventory,
    None,
}

// impl ExternalState {
//     fn run(&self, event: Event) {
//         // loop {}
//         match (self, event) {
//             (Init, Event::AuthorizeAttempt) => {}
//             (Idle, Event::None) => {}
//             (Sync, Event::None) => {}
//             (Download, Event::None) => {}
//             (ArtifactInstall, Event::None) => {}
//             (ArtifactReboot, Event::None) => {}
//             (ArtifactCommit, Event::None) => {}
//             (ArtifactRollback, Event::None) => {}
//             (ArtifactRollbackReboot, Event::None) => {}
//             (ArtifactFailure, Event::None) => {}
//         }
//     }
// }

enum InternalState {
    Init,
    ArtifactInstall,
    ArtifactReboot,
}

struct Init {}

impl Init {
    fn new() -> Init {
        Init {}
    }
    // Runs the sub-state machine for the external init states.
    // fn run(&self) {
    //     let event = Event::Type; // Is a function which handles the state (whichever, and returns the next state to be run)
    //     match (*self, event) {
    //         (CheckWaitState, Init) => Init{},
    //         (Init, CheckWaitState) => CheckWaitState{},
    //         // etc, etc
    //     }
    // }
}

enum InitState {
    Authorize,
    AuthorizeWait,
}

impl State for Init {
    fn name<'a>(&'a self) -> &'a str {
        "Init"
    }
    fn mutate(&self, context: &Context, client: &Client) -> Box<dyn State> {
        let auth_events = authorizationevent::AuthorizationEvent::new();
        auth_events.start();
        loop {
            debug!("Looping in Init State");
            if client.is_authorized() {
                context.sync_events.start();
            } else {
                debug!("Waiting for authorization event");
                match &auth_events.next() {
                    authorizationevent::Event::AuthorizeAttempt => {
                        debug!("Received authorization event in Init");
                        // Try to authorize, if unsuccesful, wait for the next published authorization event.
                        use reqwest::StatusCode;
                        match client.authorize() {
                            Ok(mut resp) => match resp.status() {
                                StatusCode::OK => {
                                    info!("Client successfully authorized with the Mender server");
                                    let jwt =
                                        resp.text().expect("Failed to extract the respone text");
                                    info!("JWT token: {}", jwt);
                                    client.jwt_token = jwt;
                                    break;
                                }
                                _ => {
                                    info!("Failed to authorize the client: {:?}", resp);
                                }
                            },
                            Err(e) => {
                                debug!("Authorization request error: {:?}", e);
                            }
                        }
                        debug!("Client did not authorize... Retrying");
                    }
                }
            }
        }
        Box::new(Idle {})
    }
}

struct Idle {}

impl From<Init> for Idle {
    fn from(state: Init) -> Self {
        Idle {}
    }
}

impl State for Idle {
    fn name<'a>(&'a self) -> &'a str {
        "Idle"
    }
    fn mutate(&self, context: &Context, client: &Client) -> Box<dyn State> {
        match context.sync_events.next() {
            syncevent::Events::InventoryUpdate => {
                Box::new(Sync::new(SyncState::InventoryUpdateState)) // TODO -- How to send the different transitions and sync-sub-states?
            }
            syncevent::Events::CheckForUpdate => Box::new(Sync::new(SyncState::CheckUpdateState)),
        }
    }
}

enum SyncState {
    InventoryUpdateState,
    CheckUpdateState,
}

struct Sync {
    substate: SyncState,
}

impl Sync {
    fn new(substate: SyncState) -> Sync {
        Sync { substate: substate }
    }
}

impl State for Sync {
    fn name<'a>(&'a self) -> &'a str {
        "Sync"
    }
    fn mutate(&self, context: &Context, client: &Client) -> Box<dyn State> {
        match self.substate {
            SyncState::InventoryUpdateState => Box::new(Idle {}),
            SyncState::CheckUpdateState => Box::new(Idle {}),
            _ => Box::new(Idle {}),
        }
    }
}

struct Context {
    sync_events: syncevent::Event,
}

struct StateMachine {
    external_state: ExternalState,
    internal_state: InternalState,
    state: Box<dyn State>,
    context: Context,
}

// TODO -- how to send different objects through the same interface?
// trait Event {
//     fn new() -> Self;
//     fn start(&self) -> Self;
// }

// authorizationevent module produces authorization events at a given interval for
// the state machine to consume.
mod authorizationevent {
    use std::sync::mpsc;
    use std::thread;
    use std::time;
    pub enum Event {
        AuthorizeAttempt,
    }
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
            thread::spawn(move || loop {
                thread::sleep(interval);
                match tx1.send(Event::AuthorizeAttempt) {
                    Ok(_) => println!("Successfully sent Authorization attemp Event"),
                    Err(e) => println!("Failed to send the authorization attempt Event: {}", e),
                }
            });
        }
        // Reads from the receiving end of the event channel,
        // and returns the next scheduled event. If noone are ready, it blocks.
        pub fn next(&self) -> Event {
            self.events.recv().unwrap()
        }
    }
}

impl StateMachine {
    fn new() -> StateMachine {
        StateMachine {
            external_state: ExternalState::Init,
            internal_state: InternalState::Init,
            state: Box::new(Init::new()),
            context: Context {
                sync_events: syncevent::Event::new(), // Do not start until the client is authorized!
            },
        }
    }

    pub fn run(&self) -> Result<(), &'static str> {
        let mut cur_state: Box<dyn State> = Box::new(Init::new());
        // let mut next_state: Box<dyn State>;
        let client = Client::new();
        debug!("Running the state machine");
        let next_state: Box<dyn State>;
        // loop {
        //     // Enter Current State Transition
        //     println!("StateMachine: cur_state: {}", cur_state.name());
        //     next_state = cur_state.mutate(&self.context, &client);
        //     cur_state = next_state;
        //     // Leave Current State Transition
        // }
        // First the client needs to authorize with the server
        cur_state.mutate(&self.context, &client);
        Ok(())
    }
}

fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    debug!("Starting Mender...");
    let _state_machine_res = StateMachine::new().run();
}
