use log::{debug, info, trace, warn};
use std::process::Command;
use std::time;
use reqwest::StatusCode;

use mender_artifact::*;

// use rsa::{PublicKey, RSAPrivateKey, PaddingScheme};
// use rand::rngs::OsRng;

mod client;
mod syncevent; // Bring the syncevent module into scope // Bring the client into scope
mod authevent;
use client::Client;
mod bootflags;
use bootflags::BootFlag;


pub trait EventProducer {
    fn next(&self) -> Event;
}

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

#[derive(Clone, Debug)]
pub enum Event {
    None,
    Uninitialized,
    AuthorizeAttempt,
    CheckForUpdate,
    SendInventory,
    DownloadUpdate(client::UpdateInfo),
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

struct InitState {}

impl InitState {
    fn new() -> InitState {
        InitState {}
    }

    // Check if we are in a committed, or un-committed partition
    fn is_committed() -> bool {
        let mut flgs = Vec::new();
        flgs.push("bootcount".to_string());
        flgs.push("upgrade_available".to_string());
        let flags = bootflags::BootFlag::new()
            .get(flgs);
        // Check if we are on the committed, or the uncommitted partition
        let bootcount = flags.get("bootcount").unwrap();
        let upgrade_available = flags.get("upgrade_available").unwrap();
        match (bootcount.as_ref(), upgrade_available.as_ref()) {
            ("1", "1") => {
                debug!("Entry into an uncommitted partition detected!");
                false
            }
            ("1", "0") => {
                debug!("Entry into a committed partition detected!");
                true
            }
            ("0", "0") => {
                debug!("Entry into a committed partition detected!");
                true
            }
            (_, _) => {
                debug!("Unknown pattern detected. Did something go wrong?");
                true
            }
        }
    }
}

impl State for InitState {
    fn name<'a>(&'a self) -> &'a str {
        "Init"
    }
}

struct Idle {}

impl Idle {
    fn wait_for_event(event_producer: &dyn EventProducer) -> (ExternalState, Event) {
        match event_producer.next() {
            Event::AuthorizeAttempt => (ExternalState::Sync, Event::AuthorizeAttempt),
            Event::SendInventory => (ExternalState::Sync, Event::SendInventory),
            Event::CheckForUpdate => (ExternalState::Sync, Event::CheckForUpdate),
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
    fn handle(client: &mut Client) -> (ExternalState, Event) {
        // Try to authorize, if unsuccesful, wait for the next published authorization event.
        match client.authorize() {
            Ok(mut resp) => match resp.status() {
                StatusCode::OK => {
                    info!("Client successfully authorized with the Mender server");
                    let jwt = resp.text().expect("Failed to extract the respone text");
                    info!("JWT token: {}", jwt);
                    client.jwt_token = Some(jwt);
                    client.is_authorized = true;
                    (ExternalState::Idle, Event::None)
                }
                _ => {
                    info!("Failed to authorize the client: {:?}", resp);
                    (ExternalState::Idle, Event::None)
                }
            },
            Err(e) => {
                debug!("Authorization request error: {:?}", e);
                (ExternalState::Idle, Event::None)
            }
        }
    }

    fn check_for_update(client: &mut Client) -> (ExternalState, Event) {
        match client.check_for_update() {
            Ok(mut resp) => {
                debug!("Sync: UpdateCheck: Received response");
                match resp.status() {
                    StatusCode::OK => {
                        debug!("Yay, new update!");
                        let update_info: client::UpdateInfo = resp.json().unwrap();
                        debug!("{:#?}", update_info);
                        debug!("Successfully deserialized the update response");
                        return (ExternalState::Download, Event::DownloadUpdate(update_info));
                    }
                    StatusCode::NO_CONTENT => {
                        debug!("No new update available :(");
                        return (ExternalState::Idle, Event::None);
                    }
                    _ => {
                        info!("Sync: UpdateCheck: Error checking for update");
                        return (ExternalState::Idle, Event::None);
                    }
                }
            }
            Err(e) => {
                info!("Sync: UpdateCheck: Error: {:?}", e);
                return (ExternalState::Idle, Event::None);
            }
        }
        (ExternalState::Idle, Event::None)
    }

    fn send_inventory(client: &mut Client) -> (ExternalState, Event) {
        match client.send_inventory() {
            Ok(resp) => {
                debug!("Inventory response!");
                match resp.status() {
                    StatusCode::OK => {
                        debug!("Inventory sent");
                    }
                    _ => {
                        debug!("Received non OK status code");
                    }
                }
            }
            _ => {
                debug!("Failed to send inventory");
            }
        }
        return (ExternalState::Idle, Event::None);
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

struct Download {}

impl Download {
    fn update(&self, update_info: client::UpdateInfo) -> (ExternalState, Event) {
        (ExternalState::Idle, Event::None)
    }
}

struct ArtifactInstall {}

impl ArtifactInstall {
    fn install() -> (ExternalState, Event) {
        let bf = bootflags::BootFlag::new();
        // mender_boot_part $passive_num
        //     upgrade_available 1
        //     bootcount 0
        if bf.flag("mender_boot_part", "3") // TODO -- dynamically set
            .flag("upgrade_available", "1")
            .flag("bootcount", "0")
            .set() {
                (ExternalState::ArtifactReboot, Event::None)
            }
        else {
            (ExternalState::ArtifactFailure, Event::None)
        }
    }
}

struct ArtifactReboot {}

impl ArtifactReboot {
    fn reboot() -> (ExternalState, Event) {
        if let Ok(_) =  Command::new("reboot").status() {
            debug!("Rebooting...");
        } else {
            debug!("Failed to reboot");
        }
        (ExternalState::Idle, Event::None)
    }
}

struct Context {
    // sync_events: syncevent::Event,
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
            state: Box::new(InitState::new()),
            context: Context {
                // sync_events: syncevent::Event::new(), // Do not start until the client is authorized!
            },
        }
    }

    pub fn run(&self) -> Result<(), &'static str> {
        let mut cur_state: ExternalState = ExternalState::Init;
        let mut cur_action: Event = Event::Uninitialized;
        let auth_events = authevent::AuthorizationEvent::new();
        let update_events = syncevent::SyncEvent::new();
        auth_events.start();
        let mut client = Client::new();
        debug!("Running the state machine");
        loop {
            let (state, action) = match (cur_state, cur_action) {
                (ExternalState::Init, Event::Uninitialized) => {
                    debug!("Starting the authorization event producer");
                    // (ExternalState::Idle, Event::None)
                    match InitState::is_committed() {
                        true => (ExternalState::Idle, Event::None),
                        false => (ExternalState::Idle, Event::None),
                    }
                }
                (ExternalState::Idle, _) if !client.is_authorized => {
                    debug!("Client is not authorized, waiting for authorization event");
                    Idle::wait_for_event(&auth_events)
                }
                (ExternalState::Idle, _) if client.is_authorized => {
                    debug!("Client is authorized, waiting for update event");
                    Idle::wait_for_event(&update_events)
                }
                (ExternalState::Sync, Event::AuthorizeAttempt) => {
                    debug!("Sync: Authorization attempt");
                    let (s,a) = Sync::handle(&mut client);
                    if client.is_authorized {
                        debug!("Sync: client successfully authorized. Starting the update event producer");
                        update_events.start();
                    }
                    (s,a)
                }
                (ExternalState::Sync, Event::CheckForUpdate) => {
                    debug!("Sync: Check for update");
                    Sync::check_for_update(&mut client)
                }
                (ExternalState::Sync, Event::SendInventory) => {
                    debug!("Sync: Sending inventory");
                    Sync::send_inventory(&mut client)
                }
                (ExternalState::Download, Event::DownloadUpdate(update_info)) => {
                    debug!("Download: Downloading the new update d-_-b");
                    client.download_update(update_info);
                    (ExternalState::ArtifactInstall, Event::None) // TODO
                }
                (ExternalState::ArtifactInstall, Event::None) => {
                    debug!("Install: Installing the update");
                    ArtifactInstall::install()
                }
                (ExternalState::ArtifactReboot, Event::None) => {
                    debug!("Reboot! rebooting...");
                    ArtifactReboot::reboot();
                    (ExternalState::ArtifactFailure, Event::None)
                }
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
