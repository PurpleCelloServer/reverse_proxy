// Yeahbut June 2024

use std::fs::{self, File};
use std::io::Read;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

use serde_json::Value;
use base64::{Engine as _, engine::general_purpose};
use rand::Rng;
use lazy_static::lazy_static;

// Refresh every 60 minutes
const EXPIRATION_DURATION: Duration = Duration::from_secs(3600);

struct CachedMotds {
    motd_data: Value,
    timestamp: Instant,
}

fn load_motds() -> Value {
    let file_path = "./motd.json";

    let data = match fs::read_to_string(file_path) {
        Ok(data) => data,
        Err(_) => return Value::Null,
    };

    let motd_data: Value = match serde_json::from_str(&data) {
        Ok(value) => value,
        Err(_) => return Value::Null,
    };

    motd_data
}

fn get_motds() -> Value {
    lazy_static! {
        static ref MOTDS_CACHE: Arc<Mutex<Option<CachedMotds>>> =
            Arc::new(Mutex::new(None));
    }

    let mut cache = MOTDS_CACHE.lock().unwrap();

    if let Some(cached_motds) = cache.as_ref() {
        if cached_motds.timestamp.elapsed() >= EXPIRATION_DURATION {
            println!("Refreshing MOTD cache");
            *cache = Some(CachedMotds {
                motd_data: load_motds(),
                timestamp: Instant::now(),
            });
        }
    } else {
        *cache = Some(CachedMotds {
            motd_data: load_motds(),
            timestamp: Instant::now(),
        });
    }

    let motds = cache.as_ref().unwrap().motd_data.clone();

    std::mem::drop(cache);

    motds
}

pub fn motd() -> String {
    let default = "A Minecraft Server Proxy".to_string();

    let motd_data = get_motds();

    if motd_data == Value::Null {
        return default;
    }

    let length1 = motd_data["line1"].as_array().map_or(0, |v| v.len());
    let length2 = motd_data["line2"].as_array().map_or(0, |v| v.len());

    if length1 == 0 || length2 == 0 {
        return default;
    }

    let mut rng = rand::thread_rng();
    let rand1 = rng.gen_range(0..length1) as usize;
    let rand2 = rng.gen_range(0..length2) as usize;

    let line1: &str = match motd_data["line1"][rand1].as_str() {
        Some(s) => s,
        None => return default,
    };

    // TODO: Birthdays, Holidays, and Announcements

    let line2: &str = match motd_data["line2"][rand2].as_str() {
        Some(s) => s,
        None => return default,
    };

    let line: String = format!("{}\n{}", line1, line2);
    line
}

pub fn favicon() -> Option<String> {
    let file_path = "./icon.png";

    let mut file = match File::open(file_path) {
        Ok(file) => file,
        Err(_) => return None,
    };

    let mut buffer = Vec::new();
    if let Err(_) = file.read_to_end(&mut buffer) {
        return None
    };

    let base64_string = general_purpose::STANDARD_NO_PAD.encode(buffer);
    let full_string: String =
        format!("data:image/png;base64,{}", base64_string);

    Some(full_string)
}
