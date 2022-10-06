use std::{error::Error, fmt};
use bluer::gatt::{remote::CharacteristicWriteRequest, WriteOp};
use chrono::{Local, Utc};
use derive_more::{Deref, From};

pub mod generic;
pub mod alert;
pub mod heartrate;
pub mod miband;

pub mod uuid {
    use uuid::{uuid, Uuid};

    pub const GENERIC_ACCESS:           Uuid = uuid!("00001800-0000-1000-8000-00805f9b34fb");
    pub const DEVICE_NAME:              Uuid = uuid!("00002A00-0000-1000-8000-00805f9b34fb");
    pub const APPEARANCE:               Uuid = uuid!("00002A01-0000-1000-8000-00805f9b34fb");
    pub const PRIVACY_FLAG:             Uuid = uuid!("00002A02-0000-1000-8000-00805f9b34fb");
    pub const PREFERED_PARAMS:          Uuid = uuid!("00002A04-0000-1000-8000-00805f9b34fb");

    pub const GENERIC_ATTRIBUTE:        Uuid = uuid!("00001801-0000-1000-8000-00805f9b34fb");
    pub const SERVICE_CHANGED:          Uuid = uuid!("00002A05-0000-1000-8000-00805f9b34fb");
}

pub const WITH_RESPONSE: &'static CharacteristicWriteRequest = &CharacteristicWriteRequest {
    offset: 0,
    op_type: WriteOp::Request,
    prepare_authorize: false,
    _non_exhaustive: (),
};

#[derive(Debug)]
pub enum WearLocation {
    Left,
    Right,
    Neck,
    Pocket,
}

#[derive(Debug, Deref, From)]
pub struct DateTime(chrono::DateTime<Utc>);

impl From<chrono::DateTime<Local>> for DateTime {
    fn from(dt: chrono::DateTime<Local>) -> Self {
        Self(dt.with_timezone(&chrono::Utc))
    }
}

#[derive(Clone, Copy)]
pub enum Sex {
    Male,
    Female
}

pub mod alarm {
    pub const ONCE:      u8 = 0;
    pub const MONDAY:    u8 = 1;
    pub const TUESDAY:   u8 = 2;
    pub const WEDNESDAY: u8 = 4;
    pub const THURSDAY:  u8 = 8;
    pub const FRIDAY:    u8 = 16;
    pub const SATURDAY:  u8 = 32;
    pub const SUNDAY:    u8 = 64;

    pub const WORKWEEK:  u8 = 31;
    pub const WEEKENDS:  u8 = 96;
    pub const EVERYDAY:  u8 = 127;
}


#[derive(Debug, Copy, Clone)]
pub struct NotSupportedErr;

impl <'a>Error for NotSupportedErr {}
impl <'a>fmt::Display for NotSupportedErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Not supported")
    }
}
