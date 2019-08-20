use std::process::Command;
use std::time;

mod syncevent; // Bring the syncevent module into scope

trait State {
    fn mutate(&self, context: &Context, client: &Client) -> Box<dyn State>;
}

struct Client {
    is_authorized: bool,
}

impl Client {
    fn authorize(&self) -> bool {
        if !self.is_authorized {
            // Do authorization
            true
        } else {
            false
        }
    }
}

impl Client {
    fn is_authorized(&self) -> bool {
        return self.is_authorized;
    }
}

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
    fn mutate(&self, context: &Context, client: &Client) -> Box<dyn State> {
        if client.is_authorized() {
            context.sync_events.start();
            Box::new(Idle {})
        } else {
            match context.auth_events.next() {
                authorizationevent::Event::AuthorizeAttempt => {
                    // Try to authorize, if unsuccesful, wait for the next published authorization event.
                    Box::new(Init {})
                }
            }
            // Box::new(Init {})
        }
    }
}

struct Idle {}

impl From<Init> for Idle {
    fn from(state: Init) -> Self {
        Idle {}
    }
}

impl State for Idle {
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
    fn mutate(&self, context: &Context, client: &Client) -> Box<dyn State> {
        match self.substate {
            SyncState::InventoryUpdateState => Box::new(Idle {}),
            SyncState::CheckUpdateState => Box::new(Idle {}),
            _ => Box::new(Init {}),
        }
    }
}

struct Context {
    sync_events: syncevent::Event,
    auth_events: authorizationevent::AuthorizationEvent,
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
                tx1.send(Event::AuthorizeAttempt).unwrap();
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
                auth_events: authorizationevent::AuthorizationEvent::new(),
            },
        }
    }

    pub fn run(&self) -> Result<(), &'static str> {
        let mut cur_state: Box<dyn State> = Box::new(Init::new());
        // let mut next_state: Box<dyn State>;
        let client = &Client {
            is_authorized: false,
        };
        self.context.auth_events.start(); // Since the client is not authorized, start the authorization event publisher.
        loop {
            // Enter Current State Transition
            cur_state = cur_state.mutate(&self.context, client);
            // Leave Current State Transition
        }
    }
}

trait Enter<T> {
    fn error_state(&self) -> Box<dyn State>;
    fn next_state(&self) -> Box<dyn State>;
    // A Transition either runs successfully (ie, no error in the state scripts),
    // or fails, on which an error state is returned.
    fn enter(&self) -> Box<dyn State> {
        // run enter script
        // fn enter() -> next_state_handle
        if Command::new("Artifact_Enter").output().is_err() {
            self.error_state()
        } else {
            self.next_state()
        }
    }
}
trait Leave<T> {
    fn error_state(&self) -> Box<dyn State>;
    fn next_state(&self) -> Box<dyn State>;

    fn leave(&self) -> Box<dyn State> {
        if Command::new("Artifact_Leave").output().is_err() {
            self.error_state()
        } else {
            self.next_state()
        }
    }
}

fn main() {
    let _state_machine_res = StateMachine::new().run();
}
