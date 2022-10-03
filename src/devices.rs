use std::{error::Error, ops::Deref, pin::Pin, fmt};
use async_trait::async_trait;
use bluer::gatt::{remote::CharacteristicWriteRequest, WriteOp};
use chrono::{Local, Utc};
use futures::Stream;

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

    pub const ALERT:                    Uuid = uuid!("00001802-0000-1000-8000-00805f9b34fb");
    pub const ALERT_LEVEL:              Uuid = uuid!("00002a06-0000-1000-8000-00805f9b34fb");

    pub const HEART_RATE:               Uuid = uuid!("0000180D-0000-1000-8000-00805f9b34fb");
    pub const HEART_RATE_MEASUREMENT:   Uuid = uuid!("00002A37-0000-1000-8000-00805f9b34fb");
    pub const HEART_RATE_CONTROL_POINT: Uuid = uuid!("00002A39-0000-1000-8000-00805f9b34fb");
}

pub const WITH_RESPONSE: &'static CharacteristicWriteRequest = &CharacteristicWriteRequest {
    offset: 0,
    op_type: WriteOp::Request,
    prepare_authorize: false,
    _non_exhaustive: (),
};

pub struct HeartRateMeasure {
    pub samples: u8,
    pub average: u8,
}

#[async_trait]
pub trait HeartRate {
    const MANUAL:     [u8; 3] = [0x15, 0x2, 0x1];
    const CONTINUOUS: [u8; 3] = [0x15, 0x1, 0x0];
    const SLEEP:      [u8; 3] = [0x15, 0x0, 0x0];

    async fn set_heartrate_continuous(&self, flag: bool) -> Result<(), bluer::Error>;
    async fn set_heartrate_sleep(&self, flag: bool) -> Result<(), bluer::Error>;
    async fn measure_heartrate(&self) -> Result<HeartRateMeasure, Box<dyn Error>>;
    async fn notify_heartrate(&self) -> Result<Pin<Box<dyn Stream<Item = Vec<u8>>>>, bluer::Error>;
}

#[async_trait]
pub trait Alert {
    async fn alert(&self, level: AlertLevel) -> Result<(), bluer::Error>;
}

pub enum AlertLevel {
    Mild,
    High,
}

#[derive(Debug)]
pub enum WearLocation {
    Left,
    Right,
    Neck,
    Pocket,
}

#[derive(Debug)]
pub struct DateTime(chrono::DateTime<Utc>);

impl From<chrono::DateTime<Utc>> for DateTime {
    fn from(dt: chrono::DateTime<Utc>) -> Self {
        Self(dt)
    }
}

impl From<chrono::DateTime<Local>> for DateTime {
    fn from(dt: chrono::DateTime<Local>) -> Self {
        Self(dt.with_timezone(&chrono::Utc))
    }
}

impl Deref for DateTime {
    type Target = chrono::DateTime<Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
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
