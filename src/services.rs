use std::fmt::{self, format};
use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::time::Duration;

use futures::StreamExt;

use btleplug::api::{Peripheral, Characteristic, WriteType};

use chrono::{Utc, TimeZone, DateTime, Datelike, Timelike, Local};
use crc::{Crc, CRC_8_MAXIM_DOW};
use tokio::time::sleep;
use ::uuid::Uuid;

static CRC: Crc<u8> = Crc::<u8>::new(&CRC_8_MAXIM_DOW);

mod uuid {
    use uuid::{uuid, Uuid};

    // const base: &'static str = "-0000-1000-8000-00805f9b34fb";

    pub const ALERT:                    Uuid = uuid!("00001802-0000-1000-8000-00805f9b34fb");
    pub const ALERT_LEVEL:              Uuid = uuid!("00002a06-0000-1000-8000-00805f9b34fb");
    pub const DATE_TIME:                Uuid = uuid!("0000ff0a-0000-1000-8000-00805f9b34fb");
    pub const DEVICE_INFO:              Uuid = uuid!("0000ff01-0000-1000-8000-00805f9b34fb");
    pub const NOTIFICATIONS:            Uuid = uuid!("0000ff03-0000-1000-8000-00805f9b34fb");
    pub const CHARACTERISTIC_ACTIVITY:  Uuid = uuid!("0000ff07-0000-1000-8000-00805f9b34fb");
    pub const USER_INFO:                Uuid = uuid!("0000ff04-0000-1000-8000-00805f9b34fb");
    pub const BATTERY_INFO:             Uuid = uuid!("0000ff0c-0000-1000-8000-00805f9b34fb");
    pub const PAIR:                     Uuid = uuid!("0000ff0f-0000-1000-8000-00805f9b34fb");
    pub const HEART_RATE:               Uuid = uuid!("0000180D-0000-1000-8000-00805f9b34fb");
    pub const HEART_RATE_MEASUREMENT:   Uuid = uuid!("00002A37-0000-1000-8000-00805f9b34fb");
    pub const HEART_RATE_CONTROL_POINT: Uuid = uuid!("00002A39-0000-1000-8000-00805f9b34fb");
    pub const MAC:                      Uuid = uuid!("0000fec9-0000-1000-8000-00805f9b34fb");
}

// lazy_static! {
    // pub static ref NAMES: HashMap<Uuid, &'static str> = HashMap::from([
        // (uuid::ALERT,                    "Immediate Alert"),
        // (uuid::ALERT_LEVEL,              "Alert level"),
        // (uuid::HEART_RATE,               "Heart Rate"),
        // (uuid::HEART_RATE_MEASUREMENT,   "Heart Rate Measurement"),
        // (uuid::HEART_RATE_CONTROL_POINT, "Heart Rate Control Point"),
        // (uuid::MAC,                      "MAC Address"),
        // (uuid::DEVICE_INFO,              "Device Info"),
        // (uuid::USER_INFO,                "User Info"),
    // ]);
// }

pub enum AlertLevel {
    Mild,
    High,
}

pub struct Version([u8; 4]);

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[3], self.0[2], self.0[1], self.0[0])
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
        } else {
            Err(ParseErr)
        }
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

#[derive(Debug)]
pub enum BatteryStatus {
    Low,
    NotCharging,
    Charging,
    Full,
}

impl TryFrom<u8> for BatteryStatus {
    type Error = ParseErr;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(BatteryStatus::Low),
            2 => Ok(BatteryStatus::Charging),
            3 => Ok(BatteryStatus::Full),
            4 => Ok(BatteryStatus::NotCharging),
            _ => Err(ParseErr),
        }
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

pub struct MiBand<P: Peripheral> {
    pub device:  P,
    user:        User,
    device_info: Option<DeviceInfo>,
}

mod notifications {
    use btleplug::api::ValueNotification;
    use ::uuid::Uuid;
    use super::uuid;

    pub struct StaticValueNotification {
        uuid: Uuid,
        value: &'static [u8],
    }

    impl PartialEq<ValueNotification> for StaticValueNotification {
        fn eq(&self, other: &ValueNotification) -> bool {
            self.value == other.value && self.uuid == other.uuid
        }
    }

    pub const AUTH_SUCCEDED:  StaticValueNotification = StaticValueNotification { uuid: uuid::NOTIFICATIONS, value: &[0x05]};
    pub const AUTH_FAILED:    StaticValueNotification = StaticValueNotification { uuid: uuid::NOTIFICATIONS, value: &[0x06]};
    pub const AUTH_CONFIRMED: StaticValueNotification = StaticValueNotification { uuid: uuid::NOTIFICATIONS, value: &[0x0a]};
    pub const AUTH_AWAITING:  StaticValueNotification = StaticValueNotification { uuid: uuid::NOTIFICATIONS, value: &[0x13]};
    pub const AUTH_TIMEOUT:   StaticValueNotification = StaticValueNotification { uuid: uuid::NOTIFICATIONS, value: &[0x9]};
}


impl<P: Peripheral> MiBand<P> {
    pub async fn new(device: P, user: User) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            device,
            user,
            device_info: None,
        })
    }

    async fn device_info(&self) -> Result<DeviceInfo, Box<dyn Error>> {
        let characteristic = try_characteristic(&self.device, uuid::DEVICE_INFO)?;

        let payload = self.device.read(&characteristic).await?;

        DeviceInfo::try_from(payload.as_slice()).map_err(Box::from)
    }

    // Critical: ConnectionSupervisionTimeout=500
    pub async fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        let device = &self.device;

        loop {
            if device.is_connected().await? {
                break
            }

            if let Err(e) = device.connect().await {
                eprintln!("{e}");
            }

            sleep(Duration::from_millis(150)).await;
        }

        device.discover_services().await?;

        let characteristic = try_characteristic(device, uuid::NOTIFICATIONS)?;
        device.subscribe(&characteristic).await?;

        let characteristic = try_characteristic(device, uuid::CHARACTERISTIC_ACTIVITY)?;
        device.subscribe(&characteristic).await?;

        self.device_info = Some(self.device_info().await?);
        self.set_user().await?;

        let mut notifications = device.notifications().await?;
        let n = notifications.next().await.unwrap();

        if notifications::AUTH_FAILED == n {
            let n = notifications.next().await.unwrap();
            if notifications::AUTH_AWAITING != n {
                return Err(format!("Unexpected notification {:?}", n).into())
            }

            let n = notifications.next().await.unwrap();
            if notifications::AUTH_CONFIRMED != n {
                return Err(format!("Unexpected notification {:?}", n).into())
            }
        }
        else if notifications::AUTH_SUCCEDED != n {
            return Err(format!("Unexpected notification {:?}", n).into())
        }

        self.set_datetime(&Local::now()).await?;

        Ok(())
    }

    pub async fn set_user(&self) -> Result<(), Box<dyn Error>> {
        let device_info = match &self.device_info {
            Some(device_info) => device_info,
            None => {
                return Err("ASD".into());
            }
        };

        let characteristic = try_characteristic(&self.device, uuid::USER_INFO)?;

        let mut payload = self.user.into_bytes();
        payload[8] = 1;
        payload[9] = device_info.feature;
        payload[10] = device_info.appearance;
        payload[19] = (CRC.checksum(&payload[..19]) ^ 0xAA) as u8;

        self.device.write(&characteristic, &payload, WriteType::WithResponse).await?;

        Ok(())
    }

    async fn set_datetime(&self, dt: &DateTime<Local>) -> Result<(), Box<dyn Error>> {
        let characteristic = try_characteristic(&self.device, uuid::NOTIFICATIONS)?;

        let mut payload = [0xFF; 12];
        payload[0] = (dt.year() - 2000) as u8;
        payload[1] = dt.month() as u8;
        payload[2] = dt.day() as u8;
        payload[3] = dt.hour() as u8;
        payload[4] = dt.minute() as u8;
        payload[5] = dt.second() as u8;

        self.device.write(&characteristic, &payload, WriteType::WithoutResponse).await?;

        Ok(())
    }

    pub async fn battery(&self) -> Result<BatteryInfo, Box<dyn Error>> {
        let characteristic = try_characteristic(&self.device, uuid::BATTERY_INFO)?;

        let payload = self.device.read(&characteristic).await?;

        BatteryInfo::try_from(payload.as_slice()).map_err(Box::from)
    }

    pub async fn alert(&self, level: AlertLevel) -> Result<(), Box<dyn Error>> {
        let characteristic = try_characteristic(&self.device, uuid::PAIR)?;

        let payload = match level {
            AlertLevel::Mild => [0x1],
            AlertLevel::High => [0x2],
        };

        self.device.write(&characteristic, &payload, WriteType::WithoutResponse).await?;

        Ok(())
    }

}

#[derive(Debug, Clone, Copy)]
struct CharacteristicError(Uuid);

impl fmt::Display for CharacteristicError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "couldn't find characteristic with uuid {}", self.0)
    }
}

impl Error for CharacteristicError {}

fn try_characteristic(device: &impl Peripheral, uuid: Uuid) -> Result<Characteristic, CharacteristicError> {
    device.characteristics()
        .into_iter()
        .find(|c| c.uuid == uuid)
        .ok_or(CharacteristicError(uuid))
}


#[derive(Debug, Copy, Clone)]
pub struct ParseErr;

impl <'a>fmt::Display for ParseErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}

impl <'a>Error for ParseErr {}
