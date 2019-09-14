use log::{debug, info, trace, warn};
use std::process::{Command, Output};

use std::collections::HashMap;


pub struct BootFlag {
    variables: HashMap<String, String>, // Variables to set
}

impl BootFlag {

    pub fn new() -> BootFlag {
        BootFlag{variables: HashMap::new()}
    }

    pub fn flag(mut self, key: &str, value: &str) -> Self {
        self.variables.insert(key.to_string(), value.to_string());
        self
    }

    // TODO -- return error
    pub fn set(self) -> bool {
        // Loop all variables in the hashmap, and set them accordingly
        let s = String::new();
        let mut cmd = Command::new("fw_setenv");
        for (key, val) in self.variables {
            cmd.arg(format!("{}={}\n", key, val));
        }
        if cmd.status().expect("Failed to execute command").success() {
            debug!("Successfully set the firmware environment");
            true
        } else {
            info!("Failed to set the firmware environment");
            false
        }
    }

    pub fn fw_printenv(name: &str) -> Result<String, &'static str> {
        let mut printenv_cmd = Command::new("fw_printenv");
        printenv_cmd.arg(name);
        let output = printenv_cmd.output().expect("Failed to get fw value");
        assert!(output.status.success());
        return Ok(String::from_utf8(output.stdout).unwrap());
    }

    pub fn fw_setenv(key: &str, value: &str) -> bool {
        let output = Command::new("fw_setenv")
            .arg(format!("{}={}", key, value))
            .output()
            .expect("Failet to set fw key, value pair");
        return output.status.success();
    }
}
