use std::io::ErrorKind;

use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde::de::Unexpected;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
    #[serde(
        serialize_with = "serialize_as_base64",
        deserialize_with = "deserialize_from_base64"
    )]
    pub public_key: [u8; 256],
}

fn serialize_as_base64<S>(val: &[u8; 256], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let encoded = BASE64_STANDARD.encode(val);
    serializer.serialize_str(&encoded)
}

fn deserialize_from_base64<'de, D>(deserializer: D) -> Result<[u8; 256], D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let bytes = BASE64_STANDARD
        .decode(&s)
        .map_err(serde::de::Error::custom)?;

    bytes.as_slice()[..256].try_into().map_err(|_| {
        serde::de::Error::invalid_value(Unexpected::Str(&s), &"256-byte base64 string")
    })
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
