pub type KVError<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum FrameType {
    Get,
    Set,
    Pub,
    Sub,
}

impl FrameType {
    pub fn encode(&self) -> &str {
        match self {
            FrameType::Get => "Get",
            FrameType::Set => "Set",
            FrameType::Pub => "Pub",
            FrameType::Sub => "Sub",
        }
    }
}

impl From<&str> for FrameType {
    fn from(value: &str) -> Self {
        match value {
            "Get" => FrameType::Get,
            "Set" => FrameType::Set,
            "Pub" => FrameType::Pub,
            "Sub" => FrameType::Sub,
            _ => panic!("Invalid FrameType {value:?}"),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Frame<'a> {
    pub command: FrameType,
    pub key: &'a str,
    pub value: Option<&'a str>,
}

enum ParseState {
    Command,
    Key,
    Finish
}

impl<'a> Frame<'a> {
    pub fn deserialize(buffer: &'a str) -> Frame<'a> {
        let chars = buffer.chars().enumerate().skip_while(|(_, ch)| ch == &' ');

        let mut command_end = 0;
        let mut key_end = 0;
        let mut state = ParseState::Command;

        for (i, ch) in chars {
            if ch == ' ' {
                match state {
                    ParseState::Command => {
                        command_end = i;
                        state = ParseState::Key
                    },
                    ParseState::Key => {
                        key_end = i;
                        state = ParseState::Finish
                    },
                    ParseState::Finish => {
                        break;
                    }
                }
            }
        }

        // "Get name" has no whitespace at the end.
        key_end = if key_end == 0 { buffer.len() } else { key_end };

        let command: FrameType = (&buffer[0..command_end]).into();
        let key = &buffer[command_end + 1..key_end];
        let value = if key_end + 1 < buffer.len() {
            Some(&buffer[key_end + 1..])
        } else {
            None
        };

        Frame {
            command,
            key,
            value,
        }
    }

    pub fn serialize(self) -> String {
        let mut output = vec![
            self.command.encode(),
            self.key,
        ];

        if let Some(value) = self.value {
            output.push(value)
        }

        output.join(" ")
    }
}

#[cfg(test)]
mod zero_copy_tests {
    use super::*;

    #[test]
    fn zero_copy_1() {
        // To test that we are acutally not copying anything during parsing we construct the frame,
        // and then check that the difference in pointers between the start of key and the start of
        // buffer is what we expect.
        let buffer = "Get name";
        let frame = Frame::deserialize(buffer);
        let offset = frame.key.as_ptr() as usize - buffer.as_ptr() as usize;

        assert_eq!(offset, "Get ".len());

        // Just for testing. We can do a clone of buffer which should not have the correct offset.
        let buffer2 = buffer.to_string();
        let frame = Frame::deserialize(&buffer2);
        let offset = frame.key.as_ptr() as usize - buffer.as_ptr() as usize;

        assert_ne!(offset, "Get ".len());
    }
}

#[cfg(test)]
mod encode_decode_tests {
    use super::*;

    #[test]
    fn get_sub() {
        let f = Frame {
            command : FrameType::Get,
            key : "name",
            value : None
        };
        let serial = f.serialize();
        let x = Frame::deserialize(&serial);

        assert_eq!(Frame{
            command: FrameType::Get,
            key: "name",
            value: None
        }, x);

        // sub
        let f = Frame {
            command : FrameType::Sub,
            key : "name",
            value : None
        };
        let serial = f.serialize();
        let x = Frame::deserialize(&serial);

        assert_eq!(Frame {
            command: FrameType::Sub,
            key: "name",
            value: None
        }, x);
    }

    #[test]
    fn set_pub() {
        let command = FrameType::Set;
        let key = "name";
        let value = Some("foopler");

        {
            let frame = Frame { command: command.clone(), key, value };
            let serial = frame.serialize();
            let x = Frame::deserialize(&serial);

            assert_eq!(Frame { command, key, value}, x);
        }

        let command = FrameType::Pub;

        {
            let frame = Frame { command: command.clone(), key, value };
            let serial = frame.serialize();
            let x = Frame::deserialize(&serial);

            assert_eq!(Frame { command, key, value}, x);
        }
    }

}

