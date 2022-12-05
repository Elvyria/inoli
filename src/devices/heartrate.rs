use crate::Error;
use super::generic::Capability;

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

pub mod uuid {
    use uuid::{uuid, Uuid};

    pub const HEART_RATE:               Uuid = uuid!("0000180D-0000-1000-8000-00805f9b34fb");
    pub const HEART_RATE_MEASUREMENT:   Uuid = uuid!("00002A37-0000-1000-8000-00805f9b34fb");
    pub const HEART_RATE_CONTROL_POINT: Uuid = uuid!("00002A39-0000-1000-8000-00805f9b34fb");
}

pub enum HeartRateDevice { }
impl Capability for HeartRateDevice {}

#[async_trait]
pub trait HeartRate {
    const MANUAL:     [u8; 3] = [0x15, 0x2, 0x1];
    const CONTINUOUS: [u8; 3] = [0x15, 0x1, 0x0];
    const SLEEP:      [u8; 3] = [0x15, 0x0, 0x0];

    async fn notify_heartrate(&self) -> Result<Pin<Box<dyn Stream<Item = Vec<u8>>>>, Error>;
    async fn set_heartrate_sleep(&self, enable: bool) -> Result<(), Error>;
    async fn heartrate_continuous(&self, enable: bool) -> Result<(), Error>;
    async fn heartrate(&self) -> Result<(), Error>;
}
