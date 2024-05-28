use anyhow::{bail, Ok};

pub enum ConsoleCmd {
    Display (String),
    Play (String),
    Loop (String),
}

impl ConsoleCmd {
    pub async fn parse(input: &str) -> anyhow::Result<ConsoleCmd> {
        let (message_type, rem) = match input.find(" ") {
            Some(i) => (&input[..i], input[i+1..].trim()),
            None => (&input[..], ""),
        };

        match message_type {
            "display" => {
                Ok(ConsoleCmd::Display(rem.to_owned()))
            }
            "play" => {
                Ok(ConsoleCmd::Play(rem.to_owned()))
            }
            "loop" => {
                Ok(ConsoleCmd::Loop(rem.to_owned()))
            }
            _ => bail!("cmd not recognized"),
        }
    }
}
