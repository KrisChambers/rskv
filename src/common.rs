use std::io::{Error, ErrorKind};

use serde::{Deserialize, Serialize};

pub fn get_addr() -> String {
    "0.0.0.0:6666".to_string()
}

enum FrameType {
    Get,
    Set,
    Pub,
    Sub,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum Frame {
    Get(String),
    Set(String, String),
    Pub(String, String),
    Sub(String),
}

impl From<&str> for FrameType {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "get" => FrameType::Get,
            "set" => FrameType::Set,
            "pub" => FrameType::Pub,
            "sub" => FrameType::Sub,
            other => panic!("Unknown frame type: '{other}'"),
        }
    }
}

impl From<&String> for Frame {
    fn from(value: &String) -> Self {
        let pieces = value.split(' ').collect::<Vec<_>>();
        let frame_type: FrameType = (*pieces.first().expect("Invalid frame")).into();
        let first = *pieces.get(1).expect("Invalid Frame");

        use FrameType::*;

        match frame_type {
            Get => Frame::Get(first.trim().to_string()),
            Set => {
                let rest = *pieces.get(2).expect("Invalid Frame");
                Frame::Set(first.trim().to_string(), rest.trim().to_string())
            }
            Sub => Frame::Sub(first.trim().to_string()),
            Pub => {
                let rest = *pieces.get(2).expect("Invalid Frame");
                Frame::Pub(first.trim().to_string(), rest.trim().to_string())
            }
        }
    }
}

#[test]
fn encode_decode() {
    use bincode::*;
    let f = Frame::Get("name".into());
    let x: Frame = deserialize(&serialize(&f).unwrap()).unwrap();

    assert_eq!(Frame::Get("name".into()), x);
}

pub type KVError<T> = Result<T, Box<dyn std::error::Error>>;
