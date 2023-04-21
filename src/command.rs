use std::io::Read;

use byteorder::{ReadBytesExt, LittleEndian};
use chrono::{Utc, TimeZone};
use log::warn;

use crate::{devices::{capabilities::alert::AlertLevel, WearLocation, DateTime}, error::Error};

pub const MAGIC: &[u8; 3] = b"CMD";

#[derive(Debug)]
pub enum Command {
    Alarm(CommandAction),
    Alert(AlertLevel),
    Battery,
    DateTime(DateTime),
    Heartrate,
    HeartrateContinuous(bool),
    HeartrateSleep(bool),
    Name,
    Steps((CommandAction, Option<u32>)),
    WearLocation((CommandAction, Option<WearLocation>)),
}

#[derive(Debug)]
pub enum CommandAction {
    Get,
    Set,
}

impl Command {
    pub fn read(r: &mut impl Read) -> Result<Self, Error> {
        let kind   = r.read_u8()?;
        let action = r.read_u8().map(CommandAction::try_from)??;

        match kind {
            80  => {
                let steps = match action {
                    CommandAction::Get => None,
                    CommandAction::Set => Some(r.read_u32::<LittleEndian>()?),
                };

                Ok(Command::Steps((action, steps)))
            }
            83  => Ok(Command::Battery),
            139 => Ok(Command::Heartrate),
            173 => {
                r.read_u8()
                    .map(|n| Command::HeartrateContinuous(n != 0))
                    .map_err(Into::into)
            },
            145 => {
                r.read_u8()
                    .map(AlertLevel::try_from)?
                    .map(Command::Alert)
            },
            244 => Ok(Command::Name),
            _   => {
                warn!("Tried to parse an unknown command kind - {kind}");
                Err(Error::Nothing)
            }
        }
    }
}

impl TryFrom<u8> for CommandAction {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CommandAction::Get),
            1 => Ok(CommandAction::Set),
            _ => Err(Error::Parse { expected: "0,1", position: 0, actual: value })
        }
    }
}

fn wearlocation_from_bytes(value: u8) -> Result<WearLocation, Error> {
    match value {
        0 => Ok(WearLocation::Left),
        1 => Ok(WearLocation::Right),
        2 => Ok(WearLocation::Neck),
        3 => Ok(WearLocation::Pocket),
        _ => Err(Error::Parse { expected: "0,1,2,3", position: 0, actual: value })
    }
}

fn datetime_from_bytes(b: &[u8]) -> Result<DateTime, Error> {
    if b.len() < 6 { 
        return Err(Error::Length { expected: 6, actual: b.len() })
    }

    let year  = b[0] as i32 + 2000;
    let month = b[1] as u32;
    let day   = b[2] as u32;
    let hour  = b[3] as u32;
    let min   = b[4] as u32;
    let sec   = b[5] as u32;

    Ok(Utc.with_ymd_and_hms(year, month, day, hour, min, sec).unwrap().into())
}
