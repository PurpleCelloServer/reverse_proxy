// Yeahbut June 2024

use std::fs;
use std::time::{Duration, Instant};
use serde_json::Value;

use crate::info_messages;

const EXPIRATION_DURATION: Duration = Duration::from_secs(60);

#[derive(PartialEq)]
pub struct Player {
    pub name: String,
    pub player_uuid: Option<u128>,
    pub active: bool,
}

pub enum PlayerAllowed {
    True(Player),
    False(String),
}

#[derive(Clone)]
pub enum Whitelist {
    WhitelistOpen(WhitelistOpen),
    WhitelistFile(WhitelistFile),
}

impl Whitelist {
    pub fn check_player_whitelist(&mut self ,player: Player) -> PlayerAllowed {
        match self {
            Whitelist::WhitelistOpen(wl) => wl.check_player_whitelist(player),
            Whitelist::WhitelistFile(wl) => wl.check_player_whitelist(player),
        }
    }
}

#[derive(Clone)]
pub struct WhitelistOpen {}

impl WhitelistOpen {
    pub fn check_player_whitelist(&mut self ,player: Player) -> PlayerAllowed {
        PlayerAllowed::True(player)
    }
}

#[derive(Clone)]
pub struct WhitelistFile {
    file_path: String,
    whitelist_data: Value,
    timestamp: Instant,
}

impl WhitelistFile {
    pub fn new(file_path: String) -> Self {
        Self {
            file_path,
            whitelist_data: Value::Null,
            timestamp: Instant::now() - EXPIRATION_DURATION,
        }
    }

    fn load(&mut self) {
        let data = match fs::read_to_string(&self.file_path) {
            Ok(data) => data,
            Err(_) => "".to_string(),
        };

        self.whitelist_data = match serde_json::from_str(&data) {
            Ok(value) => value,
            Err(_) => Value::Null,
        };

        self.timestamp = Instant::now();
    }

    fn get_whitelist(&mut self) -> Vec<Player> {
        if self.timestamp.elapsed() >= EXPIRATION_DURATION {
            println!("Refreshing whitelist cache");
            self.load();
        }

        if self.whitelist_data == Value::Null {
            return Vec::new();
        }

        let whitelist_array = match self.whitelist_data.as_array() {
            Some(whitelist) => whitelist,
            None => { return Vec::new(); }
        };

        let mut whitelist: Vec<Player> = Vec::new();

        for whitelisted_player in whitelist_array {
            let player_map = match whitelisted_player.as_object() {
                Some(whitelist) => whitelist,
                None => { continue; }
            };

            let name = match player_map.get("name") {
                Some(name) => {
                    match name.as_str() {
                        Some(name) => name,
                        None => { continue; }
                    }
                },
                None => { continue; }
            };

            let player_uuid = match player_map.get("uuid") {
                Some(uuid) => {
                    match uuid.as_str() {
                        Some(uuid) => {
                            match u128::from_str_radix(uuid, 16) {
                                Ok(uuid) => uuid,
                                Err(_) => { continue; }
                            }
                        },
                        None => { continue; }
                    }
                },
                None => { continue; }
            };

            let active = match player_map.get("active") {
                Some(active) => {
                    match active.as_bool() {
                        Some(active) => active,
                        None => { false }
                    }
                },
                None => { false }
            };

            whitelist.push(Player {
                name: name.to_string(),
                player_uuid: Some(player_uuid),
                active: active,
            });
        }

        whitelist
    }

    pub fn check_player_whitelist(&mut self ,player: Player) -> PlayerAllowed {

        if player.player_uuid.is_none() {
            return PlayerAllowed::False(
                info_messages::UUID_MISSING_DISCONNECT.to_string());
        }

        let whitelist = self.get_whitelist();

        let mut invalid_uuid = false;
        let mut invalid_username = false;
        let mut is_inactive = false;

        for wl_player in whitelist {
            if wl_player.name == player.name &&
                wl_player.player_uuid == player.player_uuid {
                    if wl_player.active {
                        return PlayerAllowed::True(player);
                    } else {
                        is_inactive = true;
                    }
            } else if wl_player.name == player.name &&
                wl_player.player_uuid != player.player_uuid {
                    invalid_uuid = true;
            } else if wl_player.player_uuid == player.player_uuid &&
                wl_player.name != player.name {
                    invalid_username = true;
            }
        }

        if is_inactive {
            PlayerAllowed::False(
                info_messages::WHITELIST_STATUS_INACTIVE_DISCONNECT.to_string())
        } else if invalid_username {
            PlayerAllowed::False(
                info_messages::USERNAME_INVALID_DISCONNECT.to_string())
        } else if invalid_uuid {
            PlayerAllowed::False(
                info_messages::UUID_INVALID_DISCONNECT.to_string())
        } else {
            PlayerAllowed::False(
                info_messages::NOT_WHITELISTED_DISCONNECT.to_string())
        }
    }

}
