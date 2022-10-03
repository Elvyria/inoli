use super::*;

use std::collections::HashMap;
use std::convert::{TryInto, TryFrom};
use std::fmt::{self, format};
use std::error::Error;
use std::pin::Pin;

use async_trait::async_trait;
use bluer::gatt::remote::Characteristic;
use bluer::{Device, Address};
use chrono::{Datelike, Timelike, TimeZone, Utc, Duration};
use crc::{Crc, CRC_8_MAXIM_DOW};
use futures::{pin_mut, StreamExt, Stream, FutureExt};

pub const ADDRESS: Address = Address::new([0xC8, 0x0F, 0x10, 0x80, 0xD0, 0xAA]);

static CRC: Crc<u8> = Crc::<u8>::new(&CRC_8_MAXIM_DOW);

mod uuid {
    use uuid::{uuid, Uuid};

    pub const MI_SERVICES:              Uuid = uuid!("0000fee0-0000-1000-8000-00805f9b34fb");
    pub const DEVICE_INFO:              Uuid = uuid!("0000ff01-0000-1000-8000-00805f9b34fb");
    pub const NOTIFICATIONS:            Uuid = uuid!("0000ff03-0000-1000-8000-00805f9b34fb");
    pub const USER_INFO:                Uuid = uuid!("0000ff04-0000-1000-8000-00805f9b34fb");
    pub const CONTROL:                  Uuid = uuid!("0000ff05-0000-1000-8000-00805f9b34fb");
    pub const STEPS:                    Uuid = uuid!("0000ff06-0000-1000-8000-00805f9b34fb");
    pub const CHARACTERISTIC_ACTIVITY:  Uuid = uuid!("0000ff07-0000-1000-8000-00805f9b34fb");
    pub const LE_PARAMS:                Uuid = uuid!("0000ff09-0000-1000-8000-00805f9b34fb");
    pub const DATE_TIME:                Uuid = uuid!("0000ff0a-0000-1000-8000-00805f9b34fb");
    pub const BATTERY_INFO:             Uuid = uuid!("0000ff0c-0000-1000-8000-00805f9b34fb");
    pub const PAIR:                     Uuid = uuid!("0000ff0f-0000-1000-8000-00805f9b34fb");
    pub const MAC:                      Uuid = uuid!("0000fec9-0000-1000-8000-00805f9b34fb");

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

pub mod notifications {
    pub const ALARM:             &'static [u8] = &[0x23];
    pub const LE_PARAMS_SUCCESS: &'static [u8] = &[0x8];

    pub const AUTH_AWAITING:     &'static [u8] = &[0x13];
    pub const AUTH_CONFIRMED:    &'static [u8] = &[0x0a];
    pub const AUTH_FAILED:       &'static [u8] = &[0x6];
    pub const AUTH_SUCCESS:      &'static [u8] = &[0x5];
    pub const AUTH_TIMEOUT:      &'static [u8] = &[0x9];
}

mod command {
    use std::ops::Deref;

    pub struct Command([u8; 1]);

    impl Deref for Command {
        type Target = [u8; 1];

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl From<Command> for u8 {
        fn from(command: Command) -> Self {
            command[0]
        }
    }

    pub const ALARM:         Command = Command([0x4]);
    pub const FACTORY_RESET: Command = Command([0x9]);
    pub const SYNC:          Command = Command([0xB]);
    pub const REBOOT:        Command = Command([0xC]);
    pub const WEAR_LOCATION: Command = Command([0xF]);
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
        if !self.device.is_connected().await? {
            self.device.connect().await?;
        }

        if self.characteristics.is_empty() {
            for service in self.device.services().await? {
                let characteristics = service.characteristics().await?;

                for c in characteristics.into_iter() {
                    self.characteristics.insert(c.uuid().await?, c);
                }
            }
        }

        self.set_le_params(&LEParams::low_latency()).await?;

        Ok(())
    }

    pub async fn set_wear_location(&self, location: WearLocation) -> Result<(), bluer::Error> {
        let payload = [
            command::WEAR_LOCATION.into(),
            match location {
                WearLocation::Left  => 0,
                WearLocation::Right => 1,
                WearLocation::Neck  => 2,
                _ => panic!("Wear location is not supported {:?}", location)
            }
        ];

        self.control(payload).await
    }

    pub async fn set_alarm(&self, id: u8, dt: &DateTime, smart: bool, repeat: u8) -> Result<(), bluer::Error> {
        let mut payload = [0; 11];
        payload[0] = command::ALARM.into();
        payload[1] = id;
        payload[2] = true as u8;
        payload[3..9].copy_from_slice(&<[u8; 6]>::from(dt));
        payload[9] = smart as u8;
        payload[10] = repeat;
        
        self.control(payload).await
    }

    pub async fn authenticate(&mut self, new: bool) -> Result<(), Box<dyn Error>> {
        self.device_info = Some(self.device_info().await?);
        self.set_user(new).await
    }

    pub async fn notify_battery(&self) -> Result<impl Stream<Item = BatteryInfo>, bluer::Error> {
        let characteristic = &self.characteristics[&uuid::BATTERY_INFO];

        characteristic 
            .notify() // impl Stream<Item = Vec<u8>>
            .await
            .map(|stream|
                 stream.filter_map(|payload| async move {
                     BatteryInfo::try_from(payload.as_slice()).ok()
                 }))
    }

    pub async fn notify_characteristics(&self) -> Result<impl Stream<Item = Vec<u8>>, bluer::Error> {
        let characteristic = &self.characteristics[&uuid::CHARACTERISTIC_ACTIVITY];
        characteristic.notify().await
    }

    pub async fn notify(&self) -> Result<impl Stream<Item = Vec<u8>>, bluer::Error> {
        let characteristic = &self.characteristics[&uuid::NOTIFICATIONS];
        characteristic.notify().await
    }

    async fn device_info(&self) -> Result<DeviceInfo, Box<dyn Error>> {
        let characteristic = &self.characteristics[&uuid::DEVICE_INFO];
        let payload = characteristic.read().await?;

        DeviceInfo::try_from(payload.as_slice()).map_err(Box::from)
    }

    async fn set_user(&self, auth: bool) -> Result<(), Box<dyn Error>> {
        let device_info = match &self.device_info {
            Some(device_info) => device_info,
            None => panic!("Couldn't find device info")
        };

        let characteristic = &self.characteristics[&uuid::USER_INFO];

        let mut payload = self.user.to_bytes();
        payload[8] = auth as u8;
        payload[9] = device_info.feature;
        payload[10] = device_info.appearance;
        payload[19] = (CRC.checksum(&payload[..19]) ^ ADDRESS.last().unwrap()) as u8;

        characteristic.write_ext(&payload, WITH_RESPONSE).await?;

        Ok(())
    }

    pub async fn set_datetime(&self, dt: &DateTime) -> Result<(), bluer::Error> {
        let characteristic = &self.characteristics[&uuid::DATE_TIME];

        let mut payload = [0xFF; 12];
        payload[0..6].copy_from_slice(&<[u8; 6]>::from(dt));

        characteristic.write(&payload).await
    }

    pub async fn battery(&self) -> Result<BatteryInfo, Box<dyn Error>> {
        let characteristic = &self.characteristics[&uuid::BATTERY_INFO];

        let payload = characteristic.read().await?;

        BatteryInfo::try_from(payload.as_slice()).map_err(Box::from)
    }

    pub async fn steps(&self) -> Result<u32, Box<dyn Error>> {
        let characteristic = &self.characteristics[&uuid::STEPS];
        let payload = characteristic.read().await?;

        payload.
            try_into()
            .map(u32::from_le_bytes)
            .map_err(|_| ParseErr.into())
    }

    pub async fn notify_steps(&self) -> Result<impl Stream<Item = u32>, bluer::Error> {
        let characteristic = &self.characteristics[&uuid::STEPS];

        characteristic 
            .notify() // impl Stream<Item = Vec<u8>>
            .await
            .map(|stream|
                 stream.filter_map(|payload| async {
                     payload
                         .try_into()
                         .map(u32::from_le_bytes)
                         .ok()
                 }))
    }

    async fn le_params(&self) -> Result<LEParams, Box<dyn Error>> {
        let characteristic = &self.characteristics[&uuid::LE_PARAMS];
        let payload = characteristic.read().await?;

        LEParams::try_from(payload.as_slice()).map_err(Box::from)
    }

    async fn set_le_params(&self, params: &LEParams) -> Result<(), bluer::Error> {
        let characteristic = &self.characteristics[&uuid::LE_PARAMS];
        characteristic.write_ext(&params.to_le_bytes(), WITH_RESPONSE).await
    }

    pub async fn factory_reset(&self) -> Result<(), bluer::Error> {
        self.control(*command::FACTORY_RESET).await
    }

    pub async fn reboot(&self) -> Result<(), bluer::Error> {
        self.control(*command::REBOOT).await
    }

    async fn control<const N: usize>(&self, payload: [u8; N]) -> Result<(), bluer::Error> {
        let characteristic = &self.characteristics[&uuid::CONTROL];
        characteristic.write(&payload).await
    }

    /*  Nightly Only 
        https://github.com/rust-lang/rust/issues/76560
        
        #![feature(generic_const_exprs)]
        async fn control<const N: usize>(&self, command: command::Command, data: [u8; N]) -> Result<(), bluer::Error> {
            let payload = [u8; N + 1];
            payload[0] = command.into()
            payload[1..].copy_from_slice(&data)
        
            let characteristic = &self.characteristics[&uuid::CONTROL];
            characteristic.write(&payload).await
        }

    */
}

#[async_trait]
impl<M: Model> Alert for MiBand<M> where Self: Sync + Send {
    async fn alert(&self, level: AlertLevel) -> Result<(), bluer::Error> {
        let characteristic = &self.characteristics[&super::uuid::ALERT_LEVEL];

        let payload = match level {
            AlertLevel::Mild => [1],
            AlertLevel::High => [2],
        };

        characteristic.write(&payload).await
    }
}

#[async_trait]
impl<M: Model> HeartRate for MiBand<M> where Self: Sync + Send {

    async fn set_heartrate_continuous(&self, flag: bool) -> Result<(), bluer::Error> {
        let characteristic = &self.characteristics[&super::uuid::HEART_RATE_CONTROL_POINT];

        let mut payload = Self::CONTINUOUS;
        payload[2] = flag as u8;

        characteristic.write_ext(&payload, WITH_RESPONSE).await
    }

    async fn set_heartrate_sleep(&self, flag: bool) -> Result<(), bluer::Error> {
        let characteristic = &self.characteristics[&super::uuid::HEART_RATE_CONTROL_POINT];

        let mut payload = Self::SLEEP;
        payload[2] = flag as u8;

        characteristic.write_ext(&payload, WITH_RESPONSE).await
    }

    async fn notify_heartrate(&self) -> Result<Pin<Box<dyn Stream<Item = Vec<u8>>>>, bluer::Error> {
        let characteristic = &self.characteristics[&super::uuid::HEART_RATE_MEASUREMENT];
        characteristic
            .notify()
            .await
            .map(|stream| Box::pin(stream) as _)
    }

    async fn measure_heartrate(&self) -> Result<HeartRateMeasure, Box<dyn Error>> {
        let characteristic = &self.characteristics[&super::uuid::HEART_RATE_MEASUREMENT];
        let notifications = characteristic.notify().await?;
        pin_mut!(notifications);

        let characteristic = &self.characteristics[&super::uuid::HEART_RATE_CONTROL_POINT];
        characteristic.write_ext(&Self::MANUAL, WITH_RESPONSE).await?;

        match notifications.next().await {
            Some(v) if v.len() < 2 => todo!(),
            Some(v) => Ok(HeartRateMeasure {
                        samples: v[0],
                        average: v[1],
                    }),
            None => todo!(),
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

impl Default for User {
    fn default() -> Self {
        Self {
            id:     141279967,
            sex:    Sex::Male,
            age:    14,
            height: 162,
            weight: 54,
            alias:  "Morty".to_owned(),
        }
    }
}

impl User {
    fn to_bytes(&self) -> [u8; 20] {
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

impl TryFrom<u8> for Sex {
    type Error = ParseErr;

    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            0 => Ok(Sex::Female),
            1 => Ok(Sex::Male),
            _ => Err(ParseErr)
        }
    }
}

impl From<Sex> for u8 {
    fn from(sex: Sex) -> Self {
        match sex {
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
    pub level:       u8,
    pub datetime:    DateTime,
    pub charges:     u16,
    pub status:      BatteryStatus,
}

impl TryFrom<&[u8]> for BatteryInfo {
    type Error = ParseErr;

    fn try_from(b: &[u8]) -> Result<Self, Self::Error> {
        if b.len() == 10 {
            Ok(
                BatteryInfo {
                level:    b[0],
                datetime: DateTime::from(<[u8; 6]>::try_from(&b[1..7]).unwrap()),
                charges:  u16::from_le_bytes(b[7..9].try_into().unwrap()),
                status:   BatteryStatus::try_from(b[9])?,
            })
        } else {
            Err(ParseErr)
        }
    }
}

#[derive(Debug)]
pub enum BatteryStatus {
    Low,
    Charging,
    NotCharging,
    Full,
}

impl TryFrom<u8> for BatteryStatus {
    type Error = ParseErr;

    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            1 => Ok(BatteryStatus::Low),
            2 => Ok(BatteryStatus::Charging),
            3 => Ok(BatteryStatus::NotCharging),
            4 => Ok(BatteryStatus::Full),
            _ => Err(ParseErr)
        }
    }
}

struct LEParams {
    min_interval:           u16,
    max_interval:           u16,
    latency:                u16,
    timeout:                u16,
    connection_interval:    u16,
    advertisement_interval: u16,
}

impl Default for LEParams {
    fn default() -> Self {
        LEParams {
            min_interval:           460,
            max_interval:           500,
            latency:                0,
            timeout:                500,
            connection_interval:    0,
            advertisement_interval: 0,
        }
    }
}

impl LEParams {
    fn to_le_bytes(&self) -> [u8; 12] {
        let mut b = [0u8; 12];

        b[0..2].copy_from_slice(&self.min_interval.to_le_bytes());
        b[2..4].copy_from_slice(&self.max_interval.to_le_bytes());
        b[4..6].copy_from_slice(&self.latency.to_le_bytes());
        b[6..8].copy_from_slice(&self.timeout.to_le_bytes());
        b[8..10].copy_from_slice(&self.connection_interval.to_le_bytes());
        b[10..12].copy_from_slice(&self.advertisement_interval.to_le_bytes());

        b
    }

    fn low_latency() -> Self {
        LEParams {
            min_interval:           36,
            max_interval:           36,
            ..LEParams::default()
        }
    }
}

impl TryFrom<&[u8]> for LEParams {
    type Error = ParseErr;

    fn try_from(b: &[u8]) -> Result<Self, Self::Error> {
        if b.len() == 12 {
            Ok(LEParams {
                min_interval:           u16::from_le_bytes(b[0..2].try_into().unwrap()),
                max_interval:           u16::from_le_bytes(b[2..4].try_into().unwrap()),
                latency:                u16::from_le_bytes(b[4..6].try_into().unwrap()),
                timeout:                u16::from_le_bytes(b[6..8].try_into().unwrap()),
                connection_interval:    u16::from_le_bytes(b[8..10].try_into().unwrap()),
                advertisement_interval: u16::from_le_bytes(b[10..12].try_into().unwrap()),
            })
        } else {
            Err(ParseErr)
        }
    }
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

impl From<[u8; 6]> for DateTime {
    fn from(b: [u8; 6]) -> Self {
        Self(Utc.ymd(
                b[0] as i32 + 2000,
                b[1] as u32,
                b[2] as u32)
            .and_hms(
                b[3] as u32,
                b[4] as u32,
                b[5] as u32))
    }
}

impl From<&DateTime> for [u8; 6] {
    fn from(dt: &DateTime) -> Self {
        [
            (dt.year() - 2000) as u8,
            dt.month() as u8,
            dt.day() as u8,
            dt.hour() as u8,
            dt.minute() as u8,
            dt.second() as u8,
        ]
    }
}
