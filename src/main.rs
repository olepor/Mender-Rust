trait State {
    fn handle(&self) -> Box<dyn State>;
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

struct StateMachine {
    external_state: ExternalState,
    internal_state: InternalState,
    state: Box<dyn State>,
}

struct Init {}

impl Init {
    fn new() -> Init {
        Init {}
    }
}

impl State for Init {
    fn handle(&self) -> Box<dyn State> {
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
    fn handle(&self) -> Box<dyn State> {
        Box::new(Init {})
    }
}

struct Sync {}

impl State for Sync {
    fn handle(&self) -> Box<dyn State> {
        Box::new(Init {})
    }
}

impl StateMachine {
    fn new() -> StateMachine {
        StateMachine {
            external_state: ExternalState::Init,
            internal_state: InternalState::Init,
            state: Box::new(Init::new()),
        }
    }
}

impl StateMachine {
    pub fn run(&self) -> Result<(), &'static str> {
        let statemachine = StateMachine::new();
        let mut next_state: Box<State>;
        loop {
            next_state = statemachine.state.handle();
        }
    }
}

fn main() {
    println!("Hello, world!");
}
