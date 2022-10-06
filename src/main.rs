mod hardware;
mod devices;
mod events;

use std::error::Error;

use bluer::{AdapterEvent, Adapter, Device, Address};
use chrono::Utc;
use devices::miband::{User, MiBand, notifications, self};
use devices::WearLocation;
use futures::stream::StreamExt;
use futures::pin_mut;
use spinoff::{Spinner, Spinners};

use devices::heartrate::HeartRate;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    let device = discover(&adapter, miband::ADDRESS).await?;
    let mut miband = MiBand::new(device, User::default());
    let spinner = Spinner::new(Spinners::Dots, format!("Connecting to {}...", miband.device.address()), None);

    miband.connect().await?;

    let notifications = miband.notify().await?;
    pin_mut!(notifications);

    miband.authenticate(false).await?;

    loop {
        let n = notifications.next().await;
        match n.as_deref() {
            Some(notifications::AUTH_CONFIRMED) => break,
            Some(notifications::AUTH_SUCCESS) => break,
            Some(notifications::AUTH_AWAITING) => {
                // println!("Please confirm authentication with your device");
            }
            Some(notifications::AUTH_FAILED) => {
                miband.authenticate(true).await?;
            },
            Some(notifications::AUTH_TIMEOUT) => {
                return Err("Authentication Timeout".into())
            },
            Some(_) | None => continue,
        }
    }

    spinner.success("Connected");

    miband.set_datetime(&Utc::now().into()).await?;
    miband.set_wear_location(WearLocation::Left).await?;

    let battery_stream = miband.notify_battery().await?;
    pin_mut!(battery_stream);

    let mut heartrate_stream = miband.notify_heartrate().await?;

    miband.set_heartrate_sleep(true).await?;
    miband.heartrate_continuous(true).await?;

    let steps_stream = miband.notify_steps().await?;
    pin_mut!(steps_stream);

    loop {
        tokio::select! {
            Some(n) = notifications.next() => {
                match n.as_slice() {
                    miband::notifications::LE_PARAMS_SUCCESS => {},
                    miband::notifications::ALARM => {},
                    _ => {
                        println!("  {:?}", n)
                    }
                }
            },
            Some(battery) = battery_stream.next() => {
                println!("  {:?}", battery);
            },
            Some(heartrate) = heartrate_stream.next() => {
                println!("♥  {:?}", heartrate);
            },
            Some(steps) = steps_stream.next() => {
                println!("  {:?}", steps);
            }
            else => break
        }
    }

    Ok(())
}

async fn discover(adapter: &Adapter, lookup_address: Address) -> Result<Device, Box<dyn Error>> {
    let mut discover = adapter.discover_devices().await?;

    loop {
        if let Some(event) = discover.next().await {
            if let AdapterEvent::DeviceAdded(address) = event {
                if address == lookup_address {
                    return adapter.device(address).map_err(Box::from);
                }
            }
        }
    }
}
