use crate::Error;
use async_trait::async_trait;

use super::generic::Capability;

pub mod uuid {
    use uuid::{uuid, Uuid};

    pub const ALERT:       Uuid = uuid!("00001802-0000-1000-8000-00805f9b34fb");
    pub const ALERT_LEVEL: Uuid = uuid!("00002a06-0000-1000-8000-00805f9b34fb");
}

pub enum AlertDevice {}
impl Capability for AlertDevice {}

#[async_trait]
pub trait Alert {
    async fn alert(&self, level: AlertLevel) -> Result<(), Error>;
}

pub enum AlertLevel {
    Mild,
    High,
}
