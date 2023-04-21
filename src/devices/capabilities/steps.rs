use std::pin::Pin;

use crate::Error;

use async_trait::async_trait;
use futures::Stream;

#[async_trait]
pub trait Steps {
    async fn notify_steps(&self) -> Result<Pin<Box<dyn Stream<Item = u32> + Send>>, Error>;
    async fn set_steps(&self, steps: u32) -> Result<(), Error>;
    async fn steps(&self) -> Result<u32, Error>;
}

pub trait StepsCapable {}
