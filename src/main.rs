mod icons;
mod services;

use std::error::Error;
use std::time::Duration;
use spinoff::{Spinner, Spinners};
use tokio::task;
use tokio::time::sleep;

use futures::StreamExt;

use btleplug::api::{Central, Manager as _, Peripheral, ScanFilter, BDAddr};
use btleplug::platform::Manager;

use services::{MiBand, User, Sex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let address: BDAddr = BDAddr::from_str_delim("C8:0F:10:80:D0:AA").unwrap();

    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;

    let adapter = adapters.first().unwrap();

    let spinner = Spinner::new(Spinners::Dots, "Scanning for devices...", None);
    adapter.start_scan(ScanFilter::default()).await?;
    let device = loop {
        sleep(Duration::from_millis(100)).await;

        let peripherals = adapter.peripherals().await?;
        if let Some(device) = peripherals.into_iter().find(|p| p.address() == address) {
            break device
        }
    };
    adapter.stop_scan().await?;
    spinner.clear();

    let user = User {
        id:     141279967,
        sex:    Sex::Male,
        age:    23,
        height: 170,
        weight: 50,
        alias:  "Bob".to_owned(),
    };

    let mut miband = MiBand::new(device, user).await?;

    let spinner = Spinner::new(Spinners::Dots, format!("Connecting to {}...", miband.device.address()), None);
    miband.connect().await?;
    spinner.success("Connected");

    loop {
        let battery = miband.battery().await?;

        println!("Battery Level: {}", battery.level);

        sleep(Duration::from_secs(10)).await;
    }

    // let spinner = Spinner::new(Spinners::Dots, "Pairing...", None);
    // tokio::join!(notify(&miband.device), miband.pair());
    // spinner.clear();

    Ok(())
}

async fn notify(device: &impl Peripheral) -> Result<(), Box<dyn Error>> {
    let mut notifications = device.notifications().await?;
    while let Some(data) = notifications.next().await {
        println!("{:?}", data);
    }

    Ok(())
}
