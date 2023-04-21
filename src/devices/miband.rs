use super::bluetooth::{WITH_RESPONSE, BluetoothDevice};
use super::capabilities::alarm::AlarmFrequency;
use super::capabilities::alert::{AlertCapable, Alert};
use super::capabilities::battery::{BatteryStatus, BatteryInfo, Battery};
use super::capabilities::heartrate::{HeartRateCapable, HeartRate};
use super::capabilities::steps::Steps;
use super::{DateTime, Version, WearLocation};
use crate::bio::{Bio, Sex};
use crate::{Error, ensure_length};

use std::collections::HashMap;
use std::convert::{TryInto, TryFrom};
use std::pin::Pin;
use std::{fmt, mem};

use derive_more::Deref;
use async_trait::async_trait;
use bluer::gatt::remote::Characteristic;
use bluer::{Device, Address};
use chrono::{Datelike, Timelike, TimeZone, Utc, Local};

use crc::{Crc, CRC_8_MAXIM_DOW};
use futures::{StreamExt, Stream, pin_mut};
use log::debug;

pub const ADDRESS: Address = Address::new([0xC8, 0x0F, 0x10, 0x80, 0xD0, 0xAA]);

mod uuid {
    use uuid::{uuid, Uuid};

    pub const MI_SERVICES:              Uuid = uuid!("0000fee0-0000-1000-8000-00805f9b34fb");
    pub const DEVICE_INFO:              Uuid = uuid!("0000ff01-0000-1000-8000-00805f9b34fb");
    pub const DEVICE_NAME:              Uuid = uuid!("0000ff02-0000-1000-8000-00805f9b34fb");
    pub const NOTIFICATIONS:            Uuid = uuid!("0000ff03-0000-1000-8000-00805f9b34fb");
    pub const USER_INFO:                Uuid = uuid!("0000ff04-0000-1000-8000-00805f9b34fb");
    pub const CONTROL:                  Uuid = uuid!("0000ff05-0000-1000-8000-00805f9b34fb");
    pub const STEPS:                    Uuid = uuid!("0000ff06-0000-1000-8000-00805f9b34fb");
    pub const ACTIVITY:                 Uuid = uuid!("0000ff07-0000-1000-8000-00805f9b34fb");
    pub const LE_PARAMS:                Uuid = uuid!("0000ff09-0000-1000-8000-00805f9b34fb");
    pub const DATE_TIME:                Uuid = uuid!("0000ff0a-0000-1000-8000-00805f9b34fb");
    pub const BATTERY_INFO:             Uuid = uuid!("0000ff0c-0000-1000-8000-00805f9b34fb");
    pub const PAIR:                     Uuid = uuid!("0000ff0f-0000-1000-8000-00805f9b34fb");
    pub const MAC:                      Uuid = uuid!("0000fec9-0000-1000-8000-00805f9b34fb");

    /* Unknown 
        pub const UNKNOWN:              Uuid = uuid!("0000fee1-0000-1000-8000-00805f9b34fb");
        pub const UNKNOWN:              Uuid = uuid!("0000fedd-0000-1000-8000-00805f9b34fb");
        pub const UNKNOWN:              Uuid = uuid!("0000fede-0000-1000-8000-00805f9b34fb");
        pub const UNKNOWN:              Uuid = uuid!("0000fedf-0000-1000-8000-00805f9b34fb");
        pub const UNKNOWN:              Uuid = uuid!("0000fed0-0000-1000-8000-00805f9b34fb");
        pub const UNKNOWN:              Uuid = uuid!("0000fed1-0000-1000-8000-00805f9b34fb");
        pub const UNKNOWN:              Uuid = uuid!("0000fed2-0000-1000-8000-00805f9b34fb");
        pub const UNKNOWN:              Uuid = uuid!("0000fed3-0000-1000-8000-00805f9b34fb");
    */
}

pub mod notifications {
    pub const ALARM:             &[u8] = &[0x23];
    pub const LE_PARAMS_SUCCESS: &[u8] = &[0x8];

    pub mod auth {
        pub const AWAITING:     &[u8] = &[0x13];
        pub const CONFIRMED:    &[u8] = &[0xA];
        pub const FAILED:       &[u8] = &[0x6];
        pub const SUCCESS:      &[u8] = &[0x5];
        pub const TIMEOUT:      &[u8] = &[0x9];
    }
}

mod control {
    pub type Command = u8;

    pub const ALARM:         Command = 0x4;
    pub const STEP_GOAL:     Command = 0x5;
    pub const COLLECT_DATA:  Command = 0x6;
    pub const FACTORY_RESET: Command = 0x9;
    pub const SYNC:          Command = 0xB;
    pub const REBOOT:        Command = 0xC;
    pub const WEAR_LOCATION: Command = 0xF;
    pub const SET_STEPS:     Command = 0x14;
}

pub trait Model {}

pub enum OneS {}
impl Model for OneS {}

#[derive(Deref)]
pub struct MiBand<M: Model> {
    #[deref]
    device:      Device,
    user:        User,
    device_info: Option<DeviceInfo>,
    model:       std::marker::PhantomData<M>,
    crc:         Crc<u8>,

    // pub commands: HashMap<String, fn>
    pub characteristics: HashMap<::uuid::Uuid, Characteristic>,
}

impl<M: Model> AlertCapable for MiBand<M> {}
impl HeartRateCapable for MiBand<OneS> {}

#[async_trait]
impl BluetoothDevice for MiBand<OneS> {
    async fn connect(&mut self) -> Result<(), Error> {
        if !self.is_connected().await? {
            self.device.connect().await?;
        }

        debug!("1");

        if !self.characteristics.is_empty() {
            self.characteristics.clear()
        }

        debug!("2");

        for service in self.services().await? {
            let characteristics = service.characteristics().await?;

            for c in characteristics.into_iter() {
                let u = c.uuid().await?;
                self.characteristics.insert(u, c);
                debug!("Characteristic Found: {u}");
            }
        }

        debug!("3");

        self.set_le_params(&LEParams::low_latency()).await?;

        debug!("4");

        let characteristic = &self.characteristics[&uuid::DATE_TIME];
        characteristic.read().await?;

        debug!("5");

        self.authenticate().await?;

        debug!("6");

        self.set_datetime(&Utc::now().into()).await?;

        debug!("7");

        let today = Local::today().and_hms_opt(8, 30, 0).unwrap().into();

        self.set_alarm(0, false, &today, false, AlarmFrequency::Everyday).await?;
        self.set_alarm(1, false, &today, false, AlarmFrequency::Everyday).await?;
        self.set_alarm(2, false, &today, false, AlarmFrequency::Everyday).await?;


        debug!("8");

        Ok(())
    }

    fn characteristic(&self, uuid: ::uuid::Uuid) -> &Characteristic {
        &self.characteristics[&uuid]
    }

    fn alert(&self)     -> Option<&(dyn Alert + Sync + Send)>     { Some(self) }
    fn heartrate(&self) -> Option<&(dyn HeartRate + Sync + Send)> { Some(self) }
    fn steps(&self)     -> Option<&(dyn Steps + Sync + Send)>     { Some(self) }
    fn battery(&self)   -> Option<&(dyn Battery + Sync + Send)>   { Some(self) }
}

impl MiBand<OneS> {
    pub fn boxed(device: Device) -> Box<dyn BluetoothDevice> {
        Box::from(Self {
            device,
            user:            User::default(),
            device_info:     None,
            model:           std::marker::PhantomData::<OneS>,
            crc:             Crc::<u8>::new(&CRC_8_MAXIM_DOW),
            characteristics: HashMap::new(),
        })
    }
}

impl<M: Model> MiBand<M> where M: Sync + Send {
    pub async fn device_name(&self) -> Result<String, Error> {
        let characteristic = &self.characteristics[&uuid::DEVICE_NAME];
        let payload = characteristic.read().await?;

        Ok(String::from_utf8_lossy(&payload[3..]).to_string())
    }

    // Unknown 
    async fn _device_name(&self) -> Result<(), Error> {
        self.characteristics[&uuid::DEVICE_NAME]
            .write_ext(&[0], WITH_RESPONSE)
            .await
            .map_err(Into::into)
    }

    pub async fn set_wear_location(&self, location: WearLocation) -> Result<(), Error> {
        let payload = [
            match location {
                WearLocation::Left  => 0,
                WearLocation::Right => 1,
                WearLocation::Neck  => 2,
                _ => panic!("Wear location {:?} is not supported by the device", location)
            }
        ];

        self.control_payload(control::WEAR_LOCATION, payload).await
    }

    async fn set_steps(&self, steps: u32) -> Result<(), Error> {
        self.control_payload(control::SET_STEPS, steps.to_le_bytes()).await
    }

    pub async fn set_alarm(&self, id: u8, enabled: bool, dt: &DateTime, smart: bool, frequency: AlarmFrequency) -> Result<(), Error> {
        let mut payload = [0; 10];
        payload[0] = id;
        payload[1] = enabled as u8;
        payload[2..8].copy_from_slice(&datetime_as_bytes(dt));
        payload[8] = smart as u8;
        payload[9] = frequency.as_bits();
        
        self.control_payload(control::ALARM, payload).await
    }

    pub async fn set_step_goal(&self, steps: u16) -> Result<(), Error> {
        let mut payload = [0; 3];
        payload[1..3].copy_from_slice(&steps.to_le_bytes());

        self.control_payload(control::STEP_GOAL, payload).await
    }

    pub async fn authenticate(&mut self) -> Result<(), Error> {
        let notifications = self.notify().await?;
        pin_mut!(notifications);

        self.device_info = Some(self.device_info().await?);
        self.set_user(false).await?;

        loop {
            use notifications::auth::*;
            match notifications.next().await.as_deref() {
                Some(CONFIRMED) => { debug!("Authentication: Confirmed ✓");  break Ok(()) },
                Some(SUCCESS)   => { debug!("Authentication: Successful ✓"); break Ok(()) },
                Some(AWAITING)  => { debug!("Authentication: Awaiting confirmation...");  },
                Some(FAILED)    => { debug!("Authentication: Failed ✗");  },
                Some(TIMEOUT)   => { debug!("Authentication: Timeout "); },
                _ => continue,
            }
        }
    }

    // async fn initialization<M: Model>(miband: &MiBand<M>) -> Result<(), Error> where M: Sync + Send {
        // miband.set_datetime(&Utc::now().into()).await?;
        // miband.device_info().await?;
        // miband.battery().await?;

        // let battery_stream = miband.notify_battery().await?;
        // pin_mut!(battery_stream);

        // miband.steps().await?;

        // miband.le_params().await?;
        // miband.steps().await?;

        // let steps_stream = miband.notify_steps().await?;
        // pin_mut!(steps_stream);

        // miband.set_steps(0).await?;

        // miband.set_alarm(0, false, &Local::today().and_hms(22, 18, 0).into(), false, EVERYDAY).await?;
        // miband.set_alarm(1, false, &Local::today().and_hms(8, 30, 0).into(), true, EVERYDAY).await?;
        // miband.set_alarm(2, false, &Local::today().and_hms(8, 30, 0).into(), true, EVERYDAY).await?;

        // miband.device_info().await?;

        // miband.name_unknown().await?;

        // miband.set_wear_location(WearLocation::Left).await?;

        // // let delay = Duration::from_secs(5);
        // // let timer = tokio::time::sleep(delay);
        // // tokio::pin!(timer);

        // Ok(())
    // }

    pub async fn notify_characteristics(&self) -> Result<impl Stream<Item = Vec<u8>>, Error> {
        self.characteristics[&uuid::ACTIVITY]
            .notify()
            .await
            .map_err(Into::into)
    }

    pub async fn notify(&self) -> Result<impl Stream<Item = Vec<u8>>, Error> {
        self.characteristics[&uuid::NOTIFICATIONS]
            .notify()
            .await
            .map_err(Into::into)
    }

    pub async fn device_info(&self) -> Result<DeviceInfo, Error> {
        let characteristic = &self.characteristics[&uuid::DEVICE_INFO];
        let payload = characteristic.read().await?;

        DeviceInfo::try_from(payload.as_slice())
    }

    async fn set_user(&self, auth: bool) -> Result<(), Error> {
        let device_info = match &self.device_info {
            Some(device_info) => device_info,
            None => panic!("Couldn't find device info")
        };

        let characteristic = &self.characteristics[&uuid::USER_INFO];

        let mut payload = self.user.to_bytes();
        payload[8] = auth as u8;
        payload[9] = device_info.feature;
        payload[10] = device_info.appearance;
        payload[19] = self.crc.checksum(&payload[..19]) ^ ADDRESS.last().unwrap();

        characteristic.write_ext(&payload, WITH_RESPONSE).await?;

        Ok(())
    }

    pub async fn datetime(&self) -> Result<DateTime, Error> {
        let characteristic = &self.characteristics[&uuid::DATE_TIME];
        let payload = characteristic.read().await?;

        ensure_length!(payload, 6,
           Utc.with_ymd_and_hms(
                   payload[0] as i32 + 2000,
                   payload[1] as u32,
                   payload[2] as u32,
                   payload[3] as u32,
                   payload[4] as u32,
                   payload[5] as u32).unwrap().into())
    }

    pub async fn set_datetime(&self, dt: &DateTime) -> Result<(), Error> {
        let characteristic = &self.characteristics[&uuid::DATE_TIME];

        let mut payload = [0xFF; 12];

        payload[0..6].copy_from_slice(&datetime_as_bytes(dt));

        characteristic
            .write_ext(&payload, WITH_RESPONSE)
            .await
            .map_err(Into::into)
    }

    pub async fn le_params(&self) -> Result<LEParams, Error> {
        let characteristic = &self.characteristics[&uuid::LE_PARAMS];
        let payload = characteristic.read().await?;

        LEParams::try_from(payload.as_slice())
    }

    pub async fn set_le_params(&self, params: &LEParams) -> Result<(), Error> {
        self.characteristics[&uuid::LE_PARAMS]
            .write_ext(params.to_le_bytes(), WITH_RESPONSE)
            .await
            .map_err(Into::into)
    }

    pub async fn factory_reset(&self) -> Result<(), Error> {
        self.control(control::FACTORY_RESET).await
    }

    pub async fn reboot(&self) -> Result<(), Error> {
        self.control(control::REBOOT).await
    }

    async fn control(&self, command: control::Command) -> Result<(), Error> {
        self.characteristics[&uuid::CONTROL]
            .write_ext(&command.to_le_bytes(), WITH_RESPONSE)
            .await
            .map_err(Into::into)
    }

    async fn control_payload<const N: usize>(&self, command: control::Command, data: [u8; N]) -> Result<(), Error> {

        /* Nightly Only                                  */
        /* https://github.com/rust-lang/rust/issues/76560 */
        /* let payload = [u8; N + 1];                     */

        let mut payload = vec![0; N + 1];
        payload[0] = command;
        payload[1..].copy_from_slice(&data);

        self.characteristics[&uuid::CONTROL]
            .write_ext(&payload, WITH_RESPONSE)
            .await
            .map_err(Into::into)
    }
}

fn datetime_as_bytes(dt: &DateTime) -> [u8; 6] {
    [
        (dt.year() - 2000) as u8,
        dt.month() as u8,
        dt.day() as u8,
        dt.hour() as u8,
        dt.minute() as u8,
        dt.second() as u8,
    ]
}

#[async_trait]
impl<M: Model> Battery for MiBand<M> where Self: Sync + Send {
    async fn battery_stream(&self) -> Result<Pin<Box<dyn Stream<Item = BatteryInfo> + Send>>, Error> {
        self.characteristics[&uuid::BATTERY_INFO] 
            .notify()
            .await
            .map_err(Into::into)
            .map(|stream| stream.map(|payload| {
                     BatteryInfo::try_from(payload.as_slice()).expect("parsing battery info")
                 }))
            .map(|stream| Box::pin(stream) as _)
    }

    async fn battery(&self) -> Result<BatteryInfo, Error> {
        let characteristic = &self.characteristics[&uuid::BATTERY_INFO];

        let payload = characteristic.read().await?;

        BatteryInfo::try_from(payload.as_slice())
    }
}

#[async_trait]
impl<M: Model> Steps for MiBand<M> where Self: Sync + Send {
    async fn steps(&self) -> Result<u32, Error> {
        let characteristic = &self.characteristics[&uuid::STEPS];
        let payload = characteristic.read().await?;

        payload
            .try_into()
            .map(u32::from_le_bytes)
            .map_err(Error::vec_len::<u32>)
    }

    async fn set_steps(&self, steps: u32) -> Result<(), Error> {
        unimplemented!()
        // self.control_payload(control::SET_STEPS, steps.to_le_bytes()).await
    }

    async fn notify_steps(&self) -> Result<Pin<Box<dyn Stream<Item = u32> + Send>>, Error> {
        self.characteristics[&uuid::STEPS] 
            .notify()
            .await
            .map_err(Into::into)
            .map(|stream| stream.map(|payload| {
                     payload.try_into().map(u32::from_le_bytes).expect("parsing steps")
                 }))
            .map(|stream| Box::pin(stream) as _)
    }
}

struct User {
    id:    u32,
    alias: String,
    bio:   Bio,
}

impl Default for User {
    fn default() -> Self {
        Self {
            id:    141279967,
            alias: "Morty".to_owned(),
            bio:   Bio::default(),
        }
    }
}

impl User {
    const MAX_ALIAS_LENGTH: usize = 8;

    fn to_bytes(&self) -> [u8; 20] {
        let mut b = [0u8; 20];

        b[0..4].copy_from_slice(&self.id.to_le_bytes());
        b[4] = self.bio.sex.into();
        b[5] = self.bio.age;
        b[6] = self.bio.height;
        b[7] = self.bio.weight;

        let i = std::cmp::min(self.alias.len(), Self::MAX_ALIAS_LENGTH);
        b[11..11+i].copy_from_slice(&self.alias.as_bytes()[..i]);

        b
    }
}

impl TryFrom<u8> for Sex {
    type Error = Error;

    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            0 => Ok(Sex::Female),
            1 => Ok(Sex::Male),
            _ => Err(Error::Parse { expected: "0,1", position: 0, actual: b })
        }
    }
}

impl From<Sex> for u8 {
    fn from(sex: Sex) -> Self {
        match sex {
            Sex::Female => 0,
            Sex::Male   => 1,
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
    type Error = Error;

    fn try_from(b: &[u8]) -> Result<Self, Self::Error> {
        ensure_length!(b, 20, 
            DeviceInfo {
                id:                     u32::from_be_bytes(b[0..4].try_into().unwrap()),
                feature:                b[4],
                appearance:             b[5],
                hardware_version:       b[6],
                profile_version:        Version(b[8..12].try_into().unwrap()),
                firmware_version:       Version(b[12..16].try_into().unwrap()),
                firmware_version_heart: Version(b[16..20].try_into().unwrap()) })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self[3], self[2], self[1], self[0])
    }
}

impl TryFrom<&[u8]> for BatteryInfo {
    type Error = Error;

    // level:    u8,
    // datetime: [u8; 6],
    // charges:  u16,
    // status:   u8,

    fn try_from(b: &[u8]) -> Result<Self, Self::Error> {
        ensure_length!(b, 10,
                Self {
                level:    b[0],
                status:   BatteryStatus::try_from(b[9]).ok() })
    }
}

impl TryFrom<u8> for BatteryStatus {
    type Error = Error;

    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            1 => Ok(BatteryStatus::Low),
            2 => Ok(BatteryStatus::Charging),
            3 => Ok(BatteryStatus::NotCharging),
            4 => Ok(BatteryStatus::Full),
            _ => Err(Error::Parse { expected: "1,2,3,4", position: 0, actual: b })
        }
    }
}

#[repr(C)]
pub struct LEParams {
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
    fn to_le_bytes(&self) -> &[u8] {
        assert_eq!(mem::size_of::<Self>(), 12);

        unsafe { std::slice::from_raw_parts(self as *const Self as *const u8, mem::size_of::<Self>()) }
    }

    fn low_latency() -> Self {
        LEParams {
            min_interval: 36,
            max_interval: 36,
            ..LEParams::default()
        }
    }
}

impl TryFrom<&[u8]> for LEParams {
    type Error = Error;

    fn try_from(b: &[u8]) -> Result<Self, Self::Error> {
        ensure_length!(b, mem::size_of::<Self>(),
            unsafe { mem::transmute_copy::<[u8; mem::size_of::<Self>()], Self>(&*(b as *const _ as *const _)) }
        )
    }
}
