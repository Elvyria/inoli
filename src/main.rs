mod devices;

// use std::error::Error;

use std::time::Duration;

use bluer::{AdapterEvent};
use devices::miband::{User, Sex, MiBand};
use futures::future;
use futures::stream::{self, StreamExt};
use spinoff::{Spinner, Spinners};
use tokio::time::sleep;

#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    let mut discover = adapter.discover_devices().await?;

    let device = loop {
        if let Some(event) = discover.next().await {
            if let AdapterEvent::DeviceAdded(address) = event {
                if address == devices::miband::ADDRESS {
                    break adapter.device(address);
                }
            }
        }
    }?;

    let user = User {
        id:     141279967,
        sex:    Sex::Male,
        age:    23,
        height: 170,
        weight: 50,
        alias:  "Bob".to_owned(),
    };

    let mut miband = MiBand::new(device, user);

    let spinner = Spinner::new(Spinners::Dots, format!("Connecting to {}...", miband.device.address()), None);
    miband.connect().await.unwrap();
    spinner.success("Connected");

    Ok(())
}
