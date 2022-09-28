use std::collections::HashMap;
use std::fmt::{self, format};
use std::error::Error;

use bluer::gatt::WriteOp;
use bluer::gatt::remote::{Characteristic, CharacteristicWriteRequest};
use bluer::{Device, Address};
use chrono::{DateTime, Local, Datelike, Timelike};
use crc::{Crc, CRC_8_MAXIM_DOW};
use futures::{pin_mut, StreamExt};

pub const ADDRESS: Address = Address::new([0xC8, 0x0F, 0x10, 0x80, 0xD0, 0xAA]);

static CRC: Crc<u8> = Crc::<u8>::new(&CRC_8_MAXIM_DOW);

mod uuid {
    use uuid::{uuid, Uuid};

    // const base: &'static str = "-0000-1000-8000-00805f9b34fb";

    pub const MI_SERVICES:              Uuid = uuid!("0000Fee0-0000-1000-8000-00805f9b34fb");
    pub const DEVICE_INFO:              Uuid = uuid!("0000ff01-0000-1000-8000-00805f9b34fb");
    pub const NOTIFICATIONS:            Uuid = uuid!("0000ff03-0000-1000-8000-00805f9b34fb");
    pub const USER_INFO:                Uuid = uuid!("0000ff04-0000-1000-8000-00805f9b34fb");
    pub const CHARACTERISTIC_ACTIVITY:  Uuid = uuid!("0000ff07-0000-1000-8000-00805f9b34fb");
    pub const DATE_TIME:                Uuid = uuid!("0000ff0a-0000-1000-8000-00805f9b34fb");
    pub const BATTERY_INFO:             Uuid = uuid!("0000ff0c-0000-1000-8000-00805f9b34fb");
    pub const PAIR:                     Uuid = uuid!("0000ff0f-0000-1000-8000-00805f9b34fb");
    pub const MAC:                      Uuid = uuid!("0000fec9-0000-1000-8000-00805f9b34fb");

    pub const ALERT:                    Uuid = uuid!("00001802-0000-1000-8000-00805f9b34fb");
    pub const ALERT_LEVEL:              Uuid = uuid!("00002a06-0000-1000-8000-00805f9b34fb");

    pub const HEART_RATE:               Uuid = uuid!("0000180D-0000-1000-8000-00805f9b34fb");
    pub const HEART_RATE_MEASUREMENT:   Uuid = uuid!("00002A37-0000-1000-8000-00805f9b34fb");
    pub const HEART_RATE_CONTROL_POINT: Uuid = uuid!("00002A39-0000-1000-8000-00805f9b34fb");

    /*

    pub const UNKNOWN:                  Uuid = uuid!("0000fee1-0000-1000-8000-00805f9b34fb");
    pub const UNKNOWN:                  Uuid = uuid!("0000fedd-0000-1000-8000-00805f9b34fb");
    pub const UNKNOWN:                  Uuid = uuid!("0000fede-0000-1000-8000-00805f9b34fb");
    pub const UNKNOWN:                  Uuid = uuid!("0000fedf-0000-1000-8000-00805f9b34fb");
    pub const UNKNOWN:                  Uuid = uuid!("0000fed0-0000-1000-8000-00805f9b34fb");
    pub const UNKNOWN:                  Uuid = uuid!("0000fed1-0000-1000-8000-00805f9b34fb");
    pub const UNKNOWN:                  Uuid = uuid!("0000fed2-0000-1000-8000-00805f9b34fb");
    pub const UNKNOWN:                  Uuid = uuid!("0000fed3-0000-1000-8000-00805f9b34fb");

    */
}

mod notifications {
    pub const AUTH_SUCCEDED:  &'static [u8] = &[0x05];
    pub const AUTH_FAILED:    &'static [u8] = &[0x06];
    pub const AUTH_CONFIRMED: &'static [u8] = &[0x0a];
    pub const AUTH_AWAITING:  &'static [u8] = &[0x13];
    pub const AUTH_TIMEOUT:   &'static [u8] = &[0x9];
}

pub enum OneS {}

pub trait Model {}
impl Model for OneS {}

pub struct MiBand<M: Model> {
    pub device:      Device,
    user:            User,
    device_info:     Option<DeviceInfo>,
    model:           std::marker::PhantomData<M>,
    characteristics: HashMap<::uuid::Uuid, Characteristic>,
}

impl MiBand<OneS> {
    pub fn new(device: Device, user: User) -> Self {
        Self {
            device,
            user,
            device_info:     None,
            model:           std::marker::PhantomData,
            characteristics: HashMap::new(),
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        let device = &self.device;

        if !device.is_connected().await? {
            device.connect().await?;
        }

        let services = device.services().await?;

        for service in services {
            let characteristics = service.characteristics().await?;

            for c in characteristics.into_iter() {
                self.characteristics.insert(c.uuid().await?, c);
            }
        }

        let characteristic = &self.characteristics[&uuid::NOTIFICATIONS];
        let notifications = characteristic.notify().await?;
        pin_mut!(notifications);

        let characteristic = &self.characteristics[&uuid::CHARACTERISTIC_ACTIVITY];
        let activity = characteristic.notify().await?;
        pin_mut!(activity);

        self.device_info = Some(self.device_info().await?);
        self.set_user().await?;

        let n = notifications.next().await;
        match n.as_deref() {
            Some(notifications::AUTH_SUCCEDED) => {},
            Some(notifications::AUTH_FAILED) => {
                let n = notifications.next().await;
                if n.as_deref() != Some(notifications::AUTH_AWAITING) {
                    return Err(format!("Unexpected notification {:?}", n).into())
                }

                // TODO: Timeout
                let n = notifications.next().await;
                if n.as_deref() != Some(notifications::AUTH_CONFIRMED) {
                    return Err(format!("Unexpected notification {:?}", n).into())
                }
            },
            Some(_) | None => {
                return Err(format!("Unexpected notification {:?}", n).into())
            }
        }

        self.set_datetime(&Local::now()).await?;

        Ok(())
    }

    async fn device_info(&self) -> Result<DeviceInfo, Box<dyn Error>> {
        let characteristic = &self.characteristics[&uuid::DEVICE_INFO];
        let payload = characteristic.read().await?;

        DeviceInfo::try_from(payload.as_slice()).map_err(Box::from)
    }

    async fn set_user(&self) -> Result<(), Box<dyn Error>> {
        let device_info = match &self.device_info {
            Some(device_info) => device_info,
            None => {
                return Err("ASD".into());
            }
        };

        let characteristic = &self.characteristics[&uuid::USER_INFO];

        let mut payload = self.user.into_bytes();
        payload[8] = 1;
        payload[9] = device_info.feature;
        payload[10] = device_info.appearance;
        payload[19] = (CRC.checksum(&payload[..19]) ^ 0xAA) as u8;

        characteristic.write_ext(&payload, &CharacteristicWriteRequest {
            op_type: WriteOp::Request,
            ..CharacteristicWriteRequest::default()
        }).await?;

        Ok(())
    }

    pub async fn set_datetime(&self, dt: &DateTime<Local>) -> Result<(), Box<dyn Error>> {
        let characteristic = &self.characteristics[&uuid::DATE_TIME];

        let mut payload = [0xFF; 12];
        payload[0] = (dt.year() - 2000) as u8;
        payload[1] = dt.month() as u8;
        payload[2] = dt.day() as u8;
        payload[3] = dt.hour() as u8;
        payload[4] = dt.minute() as u8;
        payload[5] = dt.second() as u8;

        characteristic.write(&payload).await?;

        Ok(())
    }

    pub async fn battery(&self) -> Result<BatteryInfo, Box<dyn Error>> {
        let characteristic = &self.characteristics[&uuid::BATTERY_INFO];

        let payload = characteristic.read().await?;

        BatteryInfo::try_from(payload.as_slice()).map_err(Box::from)
    }

    pub async fn alert(&self, level: AlertLevel) -> Result<(), Box<dyn Error>> {
        let characteristic = &self.characteristics[&uuid::ALERT_LEVEL];

        let payload = match level {
            AlertLevel::Mild => [0x1],
            AlertLevel::High => [0x2],
        };

        characteristic.write(&payload).await?;

        Ok(())
    }
}

pub struct User {
    pub id:     u32,
    pub sex:    Sex,
    pub age:    u8,
    pub height: u8, // cm
    pub weight: u8, // kg
    pub alias:  String,
}

impl User {
    fn into_bytes(&self) -> [u8; 20] {
        let mut b = [0u8; 20];

        b[0..4].copy_from_slice(&self.id.to_le_bytes());
        b[4] = self.sex.into();
        b[5] = self.age;
        b[6] = self.height;
        b[7] = self.weight;

        let i = std::cmp::min(self.alias.len(), 8);
        b[11..11+i].copy_from_slice(&self.alias.as_bytes()[..i]);

        b
    }
}

#[derive(Clone, Copy)]
pub enum Sex {
    Male,
    Female
}

impl TryFrom<u8> for Sex {
    type Error = &'static str;

    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            0 => Ok(Sex::Female),
            1 => Ok(Sex::Male),
            _ => Err("")
        }
    }
}

impl Into<u8> for Sex {
    fn into(self) -> u8 {
        match self {
            Sex::Female => 0,
            Sex::Male => 1,
        }
    }
}

pub struct DeviceInfo {
    pub id:                     u32,
    pub feature:                u8,
    pub appearance:             u8,
    pub hardware_version:       u8,
    pub profile_version:        Version,
    pub firmware_version:       Version,
    pub firmware_version_heart: Version,
}

impl TryFrom<&[u8]> for DeviceInfo {
    type Error = ParseErr;

    fn try_from(b: &[u8]) -> Result<Self, Self::Error> {
        if b.len() == 20 {
            Ok(DeviceInfo {
                id:                     u32::from_be_bytes(b[0..4].try_into().unwrap()),
                feature:                b[4],
                appearance:             b[5],
                hardware_version:       b[6],
                profile_version:        Version(b[8..12].try_into().unwrap()),
                firmware_version:       Version(b[12..16].try_into().unwrap()),
                firmware_version_heart: Version(b[16..20].try_into().unwrap()),
            })
        }
        else { Err(ParseErr) }
    }
}

pub struct Version([u8; 4]);

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[3], self.0[2], self.0[1], self.0[0])
    }
}

#[derive(Debug)]
pub struct BatteryInfo {
    pub level:   u8,
    // pub date:    DateTime<Utc>,
    // pub charges: u16,
    // pub status:  BatteryStatus,
}

impl TryFrom<&[u8]> for BatteryInfo {
    type Error = ParseErr;

    fn try_from(b: &[u8]) -> Result<Self, Self::Error> {
        if b.len() == 10 {
            Ok(BatteryInfo {
                level:   b[0],
            })
        } else {
            Err(ParseErr)
        }
    }

    // fn try_from(b: &[u8]) -> Result<Self, Self::Error> {
        // if b.len() == 10 {
            // let date = Utc
                // .ymd(
                    // b[1] as i32 + 2000,
                    // b[2] as u32,
                    // b[3] as u32)
                // .and_hms(
                    // b[4] as u32,
                    // b[5] as u32,
                    // b[6] as u32);

            // Ok(BatteryInfo {
                // level:   b[0],
                // date,
                // charges: u16::from_be_bytes(b[7..9].try_into().unwrap()),
                // status:  BatteryStatus::try_from(b[9]).unwrap_or(BatteryStatus::Full),
            // })
        // } else {
            // Err(ParseErr)
        // }
    // }
}

#[derive(Debug)]
pub enum BatteryStatus {
    Low,
    NotCharging,
    Charging,
    Full,
}

pub enum AlertLevel {
    Mild,
    High,
}

#[derive(Debug, Clone, Copy)]
struct CharacteristicError(::uuid::Uuid);

impl Error for CharacteristicError {}
impl fmt::Display for CharacteristicError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "couldn't find characteristic with uuid {}", self.0)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ParseErr;

impl <'a>Error for ParseErr {}
impl <'a>fmt::Display for ParseErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}
