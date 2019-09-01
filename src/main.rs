use log::{debug, info, trace, warn};
use std::process::Command;
use std::time;

// use rsa::{PublicKey, RSAPrivateKey, PaddingScheme};
// use rand::rngs::OsRng;

mod client;
mod syncevent; // Bring the syncevent module into scope // Bring the client into scope
use client::Client;

trait State {
    // fn mutate(&self, context: &Context, client: &Client) -> Box<dyn State>;
    fn name<'a>(&'a self) -> &'a str;
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({})", self.name())
    }
}

// struct AuthorizationManager {

// }

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy, Debug)]
enum Event {
    None,
    Uninitialized,
    AuthorizeAttempt,
    CheckForUpdate,
    SendInventory,
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
    Authorize,
    AuthorizeWait,
    ArtifactInstall,
    ArtifactReboot,
}

struct Init {}

impl Init {
    fn new() -> Init {
        Init {}
    }

    fn run(client: &mut Client) -> ExternalState {
        // Try to authorize, if unsuccesful, wait for the next published authorization event.
        use reqwest::StatusCode;
        match client.authorize() {
            Ok(mut resp) => match resp.status() {
                StatusCode::OK => {
                    info!("Client successfully authorized with the Mender server");
                    let jwt = resp.text().expect("Failed to extract the respone text");
                    info!("JWT token: {}", jwt);
                    client.jwt_token = Some(jwt);
                    client.is_authorized = true;
                    ExternalState::Idle
                }
                _ => {
                    info!("Failed to authorize the client: {:?}", resp);
                    ExternalState::Init
                }
            },
            Err(e) => {
                debug!("Authorization request error: {:?}", e);
                ExternalState::Init
            }
        }
    }
}

enum InitState {
    Authorize,
    AuthorizeWait,
}

impl State for Init {
    fn name<'a>(&'a self) -> &'a str {
        "Init"
    }
}

struct Idle {}

impl From<Init> for Idle {
    fn from(state: Init) -> Self {
        Idle {}
    }
}

impl Idle {
    fn wait_for_event(event_producer: &dyn EventProducer) -> (ExternalState, Event) {
        match event_producer.next() {
            Event::AuthorizeAttempt => (ExternalState::Sync, Event::AuthorizeAttempt),
            _ => (ExternalState::Idle, Event::None), // Infinite loop
        }
    }
}

impl State for Idle {
    fn name<'a>(&'a self) -> &'a str {
        "Idle"
    }
    // fn mutate(&self, context: &Context, client: &Client) -> Box<dyn State> {
    //     match context.sync_events.next() {
    //         syncevent::Events::InventoryUpdate => {
    //             Box::new(Sync::new(SyncState::InventoryUpdateState)) // TODO -- How to send the different transitions and sync-sub-states?
    //         }
    //         syncevent::Events::CheckForUpdate => Box::new(Sync::new(SyncState::CheckUpdateState)),
    //     }
    // }
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
    fn handle(client: &mut Client) -> ExternalState {
        // Try to authorize, if unsuccesful, wait for the next published authorization event.
        use reqwest::StatusCode;
        match client.authorize() {
            Ok(mut resp) => match resp.status() {
                StatusCode::OK => {
                    info!("Client successfully authorized with the Mender server");
                    let jwt = resp.text().expect("Failed to extract the respone text");
                    info!("JWT token: {}", jwt);
                    client.jwt_token = Some(jwt);
                    client.is_authorized = true;
                    ExternalState::Idle
                }
                _ => {
                    info!("Failed to authorize the client: {:?}", resp);
                    ExternalState::Init
                }
            },
            Err(e) => {
                debug!("Authorization request error: {:?}", e);
                ExternalState::Init
            }
        }
    }
}

impl State for Sync {
    fn name<'a>(&'a self) -> &'a str {
        "Sync"
    }
    // fn mutate(&self, context: &Context, client: &Client) -> Box<dyn State> {
    //     match self.substate {
    //         SyncState::InventoryUpdateState => Box::new(Idle {}),
    //         SyncState::CheckUpdateState => Box::new(Idle {}),
    //         _ => Box::new(Idle {}),
    //     }
    // }
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

// authorizationevent module produces authorization events at a given interval for
// the state machine to consume.
mod authorizationevent {
    use super::Event;
    use std::sync::mpsc;
    use std::thread;
    use std::time;
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
    }

    use super::EventProducer;
    impl EventProducer for AuthorizationEvent {
        // Reads from the receiving end of the event channel,
        // and returns the next scheduled event. If noone are ready, it blocks.
        fn next(&self) -> Event {
            self.events.recv().unwrap()
        }
    }
}

trait EventProducer {
    fn next(&self) -> Event;
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
        let mut cur_state: ExternalState = ExternalState::Init;
        let mut cur_action: Event = Event::Uninitialized;
        let auth_events = authorizationevent::AuthorizationEvent::new();
        auth_events.start();
        let mut client = Client::new();
        debug!("Running the state machine");
        loop {
            let (state, action) = match (cur_state, cur_action) {
                (ExternalState::Init, Event::Uninitialized) => {
                    debug!("Starting the authorization event producer");
                    (ExternalState::Idle, Event::None)
                }
                (ExternalState::Idle, _) if !client.is_authorized => {
                    debug!("Client is not authorized, waiting for authorization event");
                    Idle::wait_for_event(&auth_events)
                }
                (ExternalState::Idle, _) if client.is_authorized => {
                    debug!("Client is authorized, waiting for authorization event");
                    Idle::wait_for_event(&auth_events) // TODO -- Replace with update and inventory events
                }
                (ExternalState::Sync, Event::AuthorizeAttempt) => {
                    match client.authorize() {
                        Ok(r) => {
                            match r.status() {
                                reqwest::StatusCode::OK => {
                                    info!("Successfully authorized with the Mender Server");
                                    client.is_authorized = true;
                                }
                                s => info!(
                                    "Failed to authorize with the Mender server: Response code: {}",
                                    s
                                ),
                            };
                            client.is_authorized = true;
                        }
                        Err(e) => {
                            info!("Failed to Authorize with the Mender Server: Error: {}", e);
                        }
                    }
                    (ExternalState::Idle, Event::None)
                }
                // (ExternalState::Sync, Event::CheckForUpdate) => ExternalState::Sync,
                // (ExternalState::Sync, Event::SendInventory) => ExternalState::Sync,
                (_, _) => panic!("Unrecognized state transition"),
            };
            debug!("cur_state: {:?}, cur_event: {:?}", state, action);
            cur_state = state;
            cur_action = action;
        }
        // First the client needs to authorize with the server
        // cur_state.mutate(&self.context, &client);
        // Ok(())
    }
}

fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    debug!("Starting Mender...");
    let _state_machine_res = StateMachine::new().run();
}
