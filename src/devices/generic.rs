use std::{error::Error, marker::PhantomData};
use std::collections::HashMap;

use derive_more::Deref;
use bluer::Uuid;
use bluer::{Device, gatt::remote::Characteristic};

pub trait Capability {}

#[derive(Deref)]
pub struct Generic<C: Capability> {
    #[deref]
    pub device:      Device,
    characteristics: HashMap<Uuid, Characteristic>,
    capability:      PhantomData<C>,
}

impl<C: Capability> Generic<C> {
    pub async fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.is_connected().await? {
            self.device.connect().await?;
        }

        if self.characteristics.is_empty() {
            for service in self.services().await? {
                let characteristics = service.characteristics().await?;

                for c in characteristics.into_iter() {
                    self.characteristics.insert(c.uuid().await?, c);
                }
            }
        }

        Ok(())
    }
}
