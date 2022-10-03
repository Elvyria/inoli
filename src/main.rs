mod hardware;
mod devices;

use std::error::Error;
use std::time::Duration;

use bluer::AdapterEvent;
use chrono::{TimeZone, Utc, Local};
use devices::miband::{User, MiBand, notifications};
use devices::{Alert, HeartRate, WearLocation, Sex, alarm};
use futures::stream::StreamExt;
use futures::pin_mut;
use spinoff::{Spinner, Spinners};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {

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
    miband.set_heartrate_continuous(true).await?;

    let steps_stream = miband.notify_steps().await?;
    pin_mut!(steps_stream);

    loop {
        tokio::select! {
            Some(n) = notifications.next() => {
                println!("  {:?}", n);
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
