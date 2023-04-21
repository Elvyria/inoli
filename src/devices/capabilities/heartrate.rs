use std::pin::Pin;

use crate::{Error, devices::bluetooth::{WITH_RESPONSE, BluetoothDevice}};

use async_trait::async_trait;
use futures::{Stream, StreamExt};

pub mod uuid {
    use uuid::{uuid, Uuid};

    pub const HEART_RATE:               Uuid = uuid!("0000180D-0000-1000-8000-00805f9b34fb");
    pub const HEART_RATE_MEASUREMENT:   Uuid = uuid!("00002A37-0000-1000-8000-00805f9b34fb");
    pub const HEART_RATE_CONTROL_POINT: Uuid = uuid!("00002A39-0000-1000-8000-00805f9b34fb");
    // 1.3.76.22 firmware version
}

const MANUAL:     [u8; 3] = [0x15, 0x2, 0x1];
const CONTINUOUS: [u8; 3] = [0x15, 0x1, 0x0];
const SLEEP:      [u8; 3] = [0x15, 0x0, 0x0];

#[async_trait]
pub trait HeartRate {
    async fn nofity_heartrate(&self) -> Result<Pin<Box<dyn Stream<Item = u8> + Send>>, Error>;
    async fn heartrate_sleep(&self, enable: bool) -> Result<(), Error>;
    async fn heartrate_continuous(&self, enable: bool) -> Result<(), Error>;
    async fn heartrate(&self) -> Result<(), Error>;
}

pub trait HeartRateCapable {}

#[async_trait] // TODO: Lookup bluetooth heartrate sensor protocol
impl<T: BluetoothDevice> HeartRate for T where Self: Sync + Send + HeartRateCapable {
    async fn nofity_heartrate(&self) -> Result<Pin<Box<dyn Stream<Item = u8> + Send>>, Error> {
        T::characteristic(self, uuid::HEART_RATE_MEASUREMENT)
            .notify()
            .await
            .map_err(Into::into)
            .map(|stream| stream.map(|v| v[1]))
            .map(|stream| Box::pin(stream) as _)
    }

    async fn heartrate_sleep(&self, enable: bool) -> Result<(), Error> {
        let characteristic = T::characteristic(self, uuid::HEART_RATE_CONTROL_POINT);

        let mut payload = SLEEP;
        payload[2] = enable as u8;

        characteristic
            .write_ext(&payload, WITH_RESPONSE)
            .await?;

        characteristic // Unknown
            .write_ext(&[0x14, 0x0], WITH_RESPONSE)
            .await
            .map_err(Into::into)
    }

    async fn heartrate_continuous(&self, enable: bool) -> Result<(), Error> {
        let mut payload = CONTINUOUS;
        payload[2] = enable as u8;

        T::characteristic(self, uuid::HEART_RATE_CONTROL_POINT)
            .write_ext(&payload, WITH_RESPONSE)
            .await
            .map_err(Into::into)
    }

    async fn heartrate(&self) -> Result<(), Error> {
        T::characteristic(self, uuid::HEART_RATE_CONTROL_POINT)
            .write_ext(&MANUAL, WITH_RESPONSE)
            .await
            .map_err(Into::into)
    }
}
