use std::pin::Pin;

use crate::Error;

use async_trait::async_trait;
use futures::Stream;

#[derive(Debug)]
pub enum BatteryStatus {
    Low,
    Charging,
    NotCharging,
    Full,
}

#[derive(Debug)]
pub struct BatteryInfo {
    pub level:  u8,
    pub status: Option<BatteryStatus>,
}

#[async_trait]
pub trait Battery {
    async fn battery_stream(&self) -> Result<Pin<Box<dyn Stream<Item = BatteryInfo> + Send>>, Error>;
    async fn battery(&self) -> Result<BatteryInfo, Error>;
}
