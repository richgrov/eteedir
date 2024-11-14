use std::io::ErrorKind;

use serde::{Deserialize, Serialize};

pub trait Packet: Serialize {
    const ID: &'static str;

    fn network_encode(&self) -> String {
        let packet_json = serde_json::to_string(&self).expect("couldn't encode packet to json");
        format!("{}|{}", Self::ID, packet_json)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessagePacket {
    pub content: String,
}

impl Packet for MessagePacket {
    const ID: &'static str = "message";
}

pub fn network_decode<'a>(raw: &'a str) -> Result<(&'a str, &'a str), std::io::Error> {
    let mut parts = raw.splitn(2, "|");
    let id = parts.next().unwrap();
    let json_data = match parts.next() {
        Some(json) => json,
        None => {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "packet didn't have '|' followed by JSON-encoded data",
            ))
        }
    };

    Ok((id, json_data))
}
