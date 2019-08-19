use serde::Deserialize;
use std::sync::mpsc;
use std::thread;
use std::time; // Multiple producer, single consumer channel.
               // syncevent creates either an InvetoryUpdateEvent, or an UpdateCheckEvent,
               // and sends them to the other asynchronous process through... Hmm...

// These are based on the config file parameters found in:
// /etc/mender/mender.conf
pub enum Events {
    InventoryUpdate,
    CheckForUpdate,
}

use std::fs::File;
use std::io::BufReader;

#[derive(serde::Deserialize, Debug)]
#[serde(default)] /* Return the default values on missing value */
struct IntervalConf {
    #[serde(rename = "UpdatePollIntervalSeconds")]
    update_check_interval: u64,
    #[serde(rename = "InventoryPollIntervalSeconds")]
    inventory_check_interval: u64,
}
impl Default for IntervalConf {
    fn default() -> Self {
        IntervalConf {
            update_check_interval: 600,
            inventory_check_interval: 1200,
        }
    }
}

// TODO -- Maybe add custom deserialization to the IntervalConf, so that
// it can be embedded as a struct to Evnt(?).
pub struct Event {
    inventory_check_interval: time::Duration,
    update_check_interval: time::Duration,
    publisher: mpsc::Sender<Events>,
    events: mpsc::Receiver<Events>,
}

impl Event {
    // Initialize the Evnt struct with an InventoryCheck at once,
    // and then an update check after a minute.
    pub fn new() -> Event {
        let file = File::open("./dummies/mender.conf").expect("Error opening file");
        let reader = BufReader::new(file);
        let conf: IntervalConf =
            serde_json::from_reader(reader).expect("Failed to parse the config file");

        let (tx1, rx) = mpsc::channel();

        Event {
            publisher: tx1,
            events: rx,
            inventory_check_interval: time::Duration::from_secs(conf.inventory_check_interval),
            update_check_interval: time::Duration::from_secs(conf.update_check_interval),
        }
    }
    // Run the event Creator loop
    pub fn start(self) -> Self {
        // Start the two asynchronous event loops,
        // and enable them to create events at the given intervals.
        let tx1 = mpsc::Sender::clone(&self.publisher);
        let update_interval = self.update_check_interval;
        thread::spawn(move || {
            loop {
                thread::sleep(update_interval);
                tx1.send(Events::CheckForUpdate).unwrap();
            }
        });
        let tx2 = mpsc::Sender::clone(&self.publisher);
        let inventory_interval = self.inventory_check_interval;
        thread::spawn(move || {
            loop {
                thread::sleep(inventory_interval);
                tx2.send(Events::InventoryUpdate).unwrap();
            }
        });
        self
    }
}
