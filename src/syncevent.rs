use serde::Deserialize;
use std::sync::mpsc;
use std::time; // Multiple producer, single consumer channel.
               // syncevent creates either an InvetoryUpdateEvent, or an UpdateCheckEvent,
               // and sends them to the other asynchronous process through... Hmm...

// These are based on the config file parameters found in:
// /etc/mender/mender.conf
enum Event {
    InventoryUpdate,
    CheckForUpdate,
}

struct InventoryUpdate {
    interval: time::Duration,
    channel: mpsc::Sender<Event>,
}

impl InventoryUpdate {
    fn new(interval: time::Duration, channel: mpsc::Sender<Event>) -> InventoryUpdate {
        InventoryUpdate { interval, channel }
    }
    fn produce(&self) -> bool {
        use std::thread;
        loop {
            thread::sleep(self.interval);
            self.channel
                .send(Event::InventoryUpdate) // Time::now ?
                .expect("UpdateCheck: Failed to send signal on the channel"); // TODO -- What to send here?
        }
    }
}

struct UpdateCheck {
    interval: time::Duration,
    channel: mpsc::Sender<Event>,
}

impl UpdateCheck {
    fn new(interval: time::Duration, channel: mpsc::Sender<Event>) -> UpdateCheck {
        UpdateCheck { interval, channel }
    }
    fn produce(&self) -> bool {
        use std::thread;
        loop {
            thread::sleep(self.interval);
            self.channel
                .send(Event::CheckForUpdate)
                .expect("UpdateCheck: Failed to send signal on the channel"); // TODO -- What to send here?
        }
    }
}

use std::fs::File;
use std::io::BufReader;

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct IntervalConf {
    update_check_interval: u64,
    inventory_check_interval: u64,
}

struct Evnt {
    publisher: mpsc::Sender<Event>,
    pub events: mpsc::Receiver<Event>,
}

impl Event {
    // Initialize the Evnt struct with an InventoryCheck at once,
    // and then an update check after a minute.
    fn new() -> Box<Evnt> {
        let file = File::open("./dummies/mender.conf").expect("Error opening file");
        let reader = BufReader::new(file);
        let conf: IntervalConf =
            serde_json::from_reader(reader).expect("Failed to parse the config file");

        let (tx1, rx) = mpsc::channel();
        Box::new(Evnt {
            publisher: tx1,
            events: rx,
        })
    }
    // Run the event Creator loop
    fn run(&self) {
        // let (tx1, rx) = mpsc::channel();
        // // Start the two asynchronous event loops,
        // // and enable them to create events at the given intervals.
        // let tx2 = mpsc::Sender::clone(&tx);
        // thread::spawn(move || {
        //     let uc = UpdateCheck::new(time.Duration::from_secs(600), tx1);
        //     uc.run();
        // }
        // );
        // thread::spawn(move || {
        //     let iu = InventoryUpdate::new(time::Duration::from_secs(1200), tx2);
        //     io.run();
        // })

    }
}
