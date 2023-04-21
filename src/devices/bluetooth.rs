use std::ops::Deref;

use async_trait::async_trait;
use bluer::gatt::WriteOp;
use bluer::gatt::remote::{Characteristic, CharacteristicWriteRequest};

use crate::Error;

use super::capabilities::alert::Alert;
use super::capabilities::battery::Battery;
use super::capabilities::heartrate::HeartRate;
use super::capabilities::steps::Steps;

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

pub const WITH_RESPONSE: &CharacteristicWriteRequest = &CharacteristicWriteRequest {
    offset: 0,
    op_type: WriteOp::Request,
    prepare_authorize: false,
    _non_exhaustive: (),
};

#[async_trait]
pub trait BluetoothDevice where Self: Sync + Send + Deref<Target = bluer::Device> {
    async fn connect(&mut self) -> Result<(), Error>;

    fn characteristic(&self, uuid: ::uuid::Uuid) -> &Characteristic;
    // fn command(&self, command: Command) -> Result<(), Error>;

    fn alert(&self)     -> Option<&(dyn Alert + Sync + Send)>;
    fn heartrate(&self) -> Option<&(dyn HeartRate + Sync + Send)>;
    fn steps(&self)     -> Option<&(dyn Steps + Sync + Send)>;
    fn battery(&self)   -> Option<&(dyn Battery + Sync + Send)>;
}
