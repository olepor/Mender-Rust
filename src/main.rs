use std::process::Command;

mod syncevent; // Bring the syncevent module into scope

trait State {
    fn mutate(&self, context: &Context) -> Box<dyn State>;
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

impl State for Init {
    fn mutate(&self, context: &Context) -> Box<dyn State> {
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
    fn mutate(&self, context: &Context) -> Box<dyn State> {
        // Check if the client is authorized
        // TODO
        // Read from the event producer, and do either a
        // CheckUpdate, or a InventoryUpdate state
        match context.sync_events.recv().unwrap() {
            syncevent::Events::InventoryUpdate => {
                Box::new(Sync::new(SyncState::InventoryUpdateState)) // TODO -- How to send the different transitions and sync-sub-states?
            }
            syncevent::Events::CheckForUpdate => Box::new(Sync::new(SyncState::CheckUpdateState)),
            _ => Box::new(Init {}),
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
    fn mutate(&self, context: &Context) -> Box<dyn State> {
        match self.substate {
            SyncState::InventoryUpdateState => Box::new(Idle{}),
            SyncState::CheckUpdateState => Box::new(Idle{}),
            _ => Box::new(Init{}),
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

impl StateMachine {
    fn new() -> StateMachine {
        StateMachine {
            external_state: ExternalState::Init,
            internal_state: InternalState::Init,
            state: Box::new(Init::new()),
            context: Context {
                sync_events: syncevent::Event::new().start(),
            },
        }
    }

    pub fn run(&self) -> Result<(), &'static str> {
        let mut cur_state: Box<dyn State> = Box::new(Init::new());
        // let mut next_state: Box<dyn State>;
        loop {
            // Enter Current State Transition
            cur_state = cur_state.mutate(&self.context);
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
