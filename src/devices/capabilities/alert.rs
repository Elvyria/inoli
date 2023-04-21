use crate::{Error, devices::bluetooth::BluetoothDevice};
use async_trait::async_trait;

pub mod uuid {
    use uuid::{uuid, Uuid};

    pub const ALERT:       Uuid = uuid!("00001802-0000-1000-8000-00805f9b34fb");
    pub const ALERT_LEVEL: Uuid = uuid!("00002a06-0000-1000-8000-00805f9b34fb");
}

#[async_trait]
pub trait Alert {
    async fn alert(&self, level: AlertLevel) -> Result<(), Error>;
}

pub trait AlertCapable {}

#[derive(Debug)]
pub enum AlertLevel {
    Mild,
    High,
}

impl TryFrom<u8> for AlertLevel {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(AlertLevel::Mild),
            2 => Ok(AlertLevel::High),
            _ => Err(Error::Parse { expected: "1,2", position: 0, actual: value })
        }
    }
}

#[async_trait]
impl<T: BluetoothDevice> Alert for T where Self: Sync + Send + AlertCapable {
    async fn alert(&self, level: AlertLevel) -> Result<(), Error> {
        let payload = match level {
            AlertLevel::Mild => [1],
            AlertLevel::High => [2],
        };

        T::characteristic(self, uuid::ALERT_LEVEL)
            .write(&payload)
            .await
            .map_err(Into::into)
    }
}

