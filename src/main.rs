mod hardware;
mod devices;

use std::time::Duration;

use thiserror::Error;
use bluer::{ErrorKind, AdapterEvent, Adapter, Device, Address};
use chrono::Utc;
use devices::miband::{User, MiBand, LEParams, notifications, self};
use devices::WearLocation;
use futures::stream::StreamExt;
use futures::pin_mut;

use devices::heartrate::HeartRate;
use tokio::time::Instant;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Bluetooth(#[from] bluer::Error),

    #[error("invalid byte at {position:#x} (expected {expected}, got {actual})")]
    Parse { expected: &'static str, position: usize, actual: u8 },

    #[error("invalid data length (expected {expected}, got {actual})")]
    Length { expected: usize, actual: usize },
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    loop {
        if let Err(Error::Bluetooth(e)) = stay_alive(&adapter, miband::ADDRESS).await {
            match e.kind {
                ErrorKind::NotFound | ErrorKind::Failed | ErrorKind::ServicesUnresolved => {
                    eprintln!("{:?}", e)
                },
                _ => {
                    return Err(Box::from(e))
                }
            }
        }
    }
}

async fn stay_alive(adapter: &Adapter, address: Address) -> Result<(), Error> {
    let device = discover(adapter, address).await?;
    let mut miband = MiBand::new(device, User::default());
    let spinner = Spinner::new(Spinners::Dots, format!("Connecting to {}...", miband.device.address()), None);

    miband.connect().await?;

    let notifications = miband.notify().await?;
    pin_mut!(notifications);

    let characteristic = miband.notify_characteristics().await?;
    pin_mut!(characteristic);

    miband.authenticate(false).await?;

    loop {
        let n = notifications.next().await;
        match n.as_deref() {
            Some(notifications::AUTH_CONFIRMED) => {
                println!("CONFIRMED");
                break;
            }
            Some(notifications::AUTH_SUCCESS) => {
                println!("SUCCESS");
                break
            },
            Some(notifications::AUTH_AWAITING) => {
                println!("AWAITING")
                // println!("Please confirm authentication with your device");
            }
            Some(notifications::AUTH_FAILED) => {
                println!("FAILED");
                // miband.authenticate(true).await?;
            },
            Some(notifications::AUTH_TIMEOUT) => {
                // return Err("Authentication Timeout".into())
            },
            Some(_) | None => continue,
        }
    }

    spinner.success("Connected");

    miband.set_datetime(&Utc::now().into()).await?;
    miband.device_info().await?;
    miband.battery().await?;

    let battery_stream = miband.notify_battery().await?;
    pin_mut!(battery_stream);

    miband.steps().await?;
    miband.le_params().await?;
    miband.steps().await?;

    // miband.set_wear_location(WearLocation::Left).await?;

    let mut heartrate_stream = miband.notify_heartrate().await?;

    // miband.set_heartrate_sleep(true).await?;
    // miband.heartrate_continuous(true).await?;

    let steps_stream = miband.notify_steps().await?;
    pin_mut!(steps_stream);

    let timer = tokio::time::sleep(Duration::from_secs(60));
    tokio::pin!(timer);

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
            Some(n) = characteristic.next() =>{
                println!("{:?}", n)
            }
            Some(battery) = battery_stream.next() => {
                println!("  {:?}", battery);
            },
            Some(heartrate) = heartrate_stream.next() => {
                println!("♥  {:?}", heartrate);
            },
            Some(steps) = steps_stream.next() => {
                println!("  {:?}", steps);
            }
            () = &mut timer => {
                miband.heartrate().await?;

                timer.as_mut().reset(Instant::now() + Duration::from_secs(60))
            }
            else => break
        }
    }

    Ok(())
}

async fn discover(adapter: &Adapter, address: Address) -> Result<Device, bluer::Error> {
    let mut discover = adapter.discover_devices().await?;

    loop {
        if let Some(AdapterEvent::DeviceAdded(discovered)) = discover.next().await {
            if discovered == address {
                return adapter.device(address);
            }
        }
    }
}
