/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::SystemTime;
use std::{collections::HashMap, sync::Mutex};

use log::{debug, info, trace, warn};
use tap::TapFallible;
use value::{DataError, Value};

type Result<T> = std::result::Result<T, std::io::Error>;

#[derive(Debug)]
pub struct CounterDb {
    counter_file: PathBuf,
    old_state: HashMap<String, (SystemTime, u64)>,
    new_state: Mutex<HashMap<String, (SystemTime, u64)>>,
}

impl CounterDb {
    pub fn new(path: PathBuf) -> Self {
        Self {
            counter_file: path,
            old_state: HashMap::new(),
            new_state: Mutex::new(HashMap::new()),
        }
    }
    pub async fn load(path: PathBuf) -> Result<Self> {
        let mut counters = Self::new(path);
        counters.try_load().await?;
        Ok(counters)
    }
    pub async fn try_load(&mut self) -> Result<()> {
        use tokio::fs;

        debug!("loading counterdb: {:?}", self.counter_file.display());
        self.old_state = match fs::read(&self.counter_file).await {
            Err(ref e) if e.kind() == ErrorKind::NotFound => HashMap::new(),            
            Err(e) => return Err(e),
            Ok(data) => serde_json::from_slice(&data)
                .tap_err(|e| warn!("Unable to deserialize counters: {e}"))
                .unwrap_or_default(),
        };

        Ok(())
    }

    #[cfg(feature = "blocking")]
    pub fn blocking_load(path: PathBuf) -> Result<Self> {
        let mut counters = Self::new(path);
        counters.try_blocking_load()?;
        Ok(counters)
    }
    #[cfg(feature = "blocking")]
    pub fn try_blocking_load(&mut self) -> Result<()> {
        use std::{fs::File, io::BufReader};

        debug!("loading counterdb: {:?}", self.counter_file.display());
        self.old_state = match File::open(&self.counter_file) {
            Ok(f) => serde_json::from_reader(BufReader::new(f))
                .tap_err(|e| warn!("Unable to deserialize counters: {e}"))
                .unwrap_or_default(),
            Err(ref e) if e.kind() == ErrorKind::NotFound => HashMap::new(), 
            Err(e) => return Err(e),
        };

        Ok(())
    }

    pub fn get(&self, k: &String) -> Option<&(SystemTime, u64)> {
        self.old_state.get(k)
    }
    pub fn insert(&self, k: String, v: (SystemTime, u64)) {
        self.new_state.lock().unwrap().insert(k, v);
    }

    pub fn difference(
        &self,
        key: String,
        new: u64,
        now: SystemTime,
    ) -> std::result::Result<Value, DataError> {
        let number = match self.get(&key) {
            None => Err(DataError::CounterPending),
            Some((_, old)) => {
                if &new < old {
                    Err(DataError::CounterOverflow)
                } else {
                    Ok(new - *old)
                }
            }
        }
        .map(|num| Value::Integer(num as i64));

        trace!(
            "difference of {}: {} - {:?} = {:?}",
            &key,
            new,
            self.get(&key),
            &number
        );

        self.insert(key, (now, new));
        number
    }

    pub fn counter(
        &self,
        key: String,
        new: u64,
        now: SystemTime,
    ) -> std::result::Result<Value, DataError> {
        let number = match self.get(&key) {
            None => Err(DataError::CounterPending),
            Some((then, old)) => {
                match (now.duration_since(*then), &new < old) {
                    (Ok(dur), false) => {
                        Ok((new - old) as f64 / dur.as_secs_f64())
                    }
                    _ => Err(DataError::CounterOverflow),
                }
            }
        }
        .map(Value::Float);

        trace!(
            "counter of {}: {:?} - {:?} = {:?}",
            &key,
            new,
            self.get(&key),
            &number
        );
        
        self.insert(key, (now, new));
        number
    }

    pub async fn save(&self) -> Result<()> {
        use tokio::{fs, io::AsyncWriteExt};

        info!("saving counters to {:?}", self.counter_file);
        if let Some(dir) = self.counter_file.parent() {
            fs::create_dir_all(dir).await?;
        }

        let mut f = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&self.counter_file)
            .await?;

        f.write_all(&serde_json::to_vec(&self.new_state).unwrap())
            .await
    }

    #[cfg(feature = "blocking")]
    pub fn blocking_save(&self) -> Result<()> {
        use std::fs;
        use std::io::Write;

        info!("saving counters to {:?}", self.counter_file);
        if let Some(dir) = self.counter_file.parent() {
            fs::create_dir_all(dir)?;
        }

        let mut f = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&self.counter_file)?;

        f.write_all(&serde_json::to_vec(&self.new_state).unwrap())
    }
}
