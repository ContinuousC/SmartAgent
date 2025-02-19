/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom};
use std::path::Path;

use super::error::Result;

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub last_file_id: u64,
}

impl State {
    pub fn new() -> State {
        State { last_file_id: 0 }
    }

    pub fn load(base_dir: &Path) -> Result<Self> {
        let path = base_dir.join("index.json");
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(path)?;
        file.lock_exclusive()?;

        let state = match serde_json::from_reader::<_, Self>(&file) {
            Err(_) => Self::new(),
            Ok(mut state) => {
                state.last_file_id += 1;
                state
            }
        };

        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?;
        serde_json::to_writer(&file, &state)?;

        Ok(state)
    }
}
