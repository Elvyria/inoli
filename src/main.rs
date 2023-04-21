mod hardware;
mod devices;
mod macros;
mod bio;
mod appraiser;
mod error;
mod command;
mod ipc;

use std::time::Duration;
use std::{sync::Arc, ops::DerefMut, path::Path, fs};

use std::os::unix::fs::FileTypeExt;

use clap::Parser;
use command::{Command, CommandAction};
use devices::miband;
use ipc::{Ipc, Message};
use log::{debug, warn};
use tokio::sync::mpsc::Receiver;
use self::error::Error;
use bluer::{AdapterEvent, Address, Adapter};
use futures::{stream::StreamExt, channel::mpsc};

use crate::devices::bluetooth::BluetoothDevice;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    quiet: bool,

    #[arg(short, long)]
    address: Option<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {

    let _args = Args::parse();

    enable_logging();

    let socket = Path::new("socket");
    if socket.try_exists()? {
        let meta = socket.metadata()?;
        if meta.file_type().is_socket() {
            fs::remove_file(socket)?;
        }
    }

    let _address = miband::ADDRESS;

    keep_alive("socket", None).await?;

    Ok(())
}

fn enable_logging() {
    let mut log_builder = env_logger::Builder::new();

    #[cfg(not(debug_assertions))] {
        log_builder.filter_level(log::LevelFilter::Warn);
    }

    #[cfg(debug_assertions)] {
        log_builder.filter_level(log::LevelFilter::Debug);
    }

    log_builder.init();
}

async fn keep_alive<P>(socket: P, address: Option<Address>) -> Result<(), Error>
where
    P: AsRef<Path>
{
    let ipc = Arc::new(Ipc::new(socket)?);

    {
        debug!("Listening for IPC clients...");
        let ipc = ipc.clone();
        tokio::spawn(async move { ipc.listen().await });
    }

    let (mut tx, rx) = mpsc::channel::<Message>(1);
    ipc.add_messenger(rx);

    let bt_session = bluer::Session::new().await?;
    let bt_adapter = bt_session.default_adapter().await?;

    let mut device = discover(&bt_adapter, address).await?;

    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        debug!("Connecting to {}", &device.address());
        device.connect().await?;

        capabilities(&ipc.clone(), device.as_ref()).await?;

        let transmitter = {
            let ipc = ipc.clone();

            tokio::spawn(async move { ipc.transmit().await.unwrap(); })
        };

        let mut commands = ipc.commands.lock().await;

        loop {
            tokio::select! {
                Ok(_) = command(device.as_ref(), &mut commands, &mut tx) => {}
                _     = interval.tick() => {
                    if !device.is_connected().await.unwrap() {
                        debug!("Lost connection to {}", device.address());
                        break
                    }
                }
                else => {
                    transmitter.abort();
                    break
                }
            }
        }
    } 
}

async fn discover(adapter: &Adapter, address: Option<Address>) -> Result<Box<dyn BluetoothDevice>, bluer::Error> {
    adapter.set_powered(true).await?;

    let mut discover = adapter.discover_devices().await?;

    debug!("Discovering devices...");

    loop {
        if let Some(AdapterEvent::DeviceAdded(discovered)) = discover.next().await {
            debug!("Discovered {}", discovered);

            if address.is_some() && Some(discovered) != address {
                continue;
            }

            let Ok(device) = adapter.device(discovered) else {
                continue
            };

            debug!("Appraising... {}", device.address());

            if let Some(bt) = appraiser::appraise(device) {
                return Ok(bt)
            }
        }
    }
}

async fn capabilities(ipc: &Ipc, device: &dyn BluetoothDevice) -> Result<(), Error> {
    debug!("Detecting device capabilities...");

    if let Some(battery) = device.battery() {
        let battery_stream = battery.battery_stream().await?;
        ipc.add_messenger(battery_stream.map(Message::from));
    }

    if let Some(steps) = device.steps() {
        let steps_stream = steps.notify_steps().await?;
        ipc.add_messenger(steps_stream.map(Message::Steps));
    }

    if let Some(heartrate) = device.heartrate() {
        let heartrate_stream = heartrate.nofity_heartrate().await?;
        ipc.add_messenger(heartrate_stream.map(Message::Heartrate));
    }

    Ok(())
}

async fn command(device: &dyn BluetoothDevice, commands: &mut impl DerefMut<Target = Receiver<Command>>, tx: &mut mpsc::Sender<Message>) -> Result<(), Error> {
    while let Some(command) = commands.recv().await {
        match command {
            Command::Steps((action, n)) => {
                if let Some(steps) = device.steps() {
                    match action {
                        CommandAction::Get => {
                            let message = steps.steps().await.map(Message::Steps)?;
                            tx.try_send(message).unwrap();
                        }
                        CommandAction::Set => {
                            steps.set_steps(n.unwrap()).await?
                        }
                    }
                }
            }
            Command::Battery => {
                if let Some(battery) = device.battery() {
                    let message = battery.battery().await.map(Message::from)?;
                    tx.try_send(message).unwrap();
                }
            }
            Command::Heartrate => {
                if let Some(heartrate) = device.heartrate() {
                    heartrate.heartrate().await?;
                }
            }
            Command::HeartrateContinuous(enable) => {
                if let Some(heartrate) = device.heartrate() {
                    heartrate.heartrate_continuous(enable).await?;
                }
            }
            Command::Name => {
                device.deref().name().await?;
            }
            _ => {
                warn!("Command is not implemented {:?}", command);
            }
        }
    }

    Ok(())
}

// Bluetooth Things
// loop {
        // match e.kind {
            // ErrorKind::Failed => {
                // match e.message.as_ref() {
                    // "le-connection-abort-by-local" => {
                        // eprintln!("Connection timeout...");
                    // }
                    // _ => eprintln!("{:?}", e)
                // }
            // },
            // ErrorKind::NotFound | ErrorKind::ServicesUnresolved => {
                // eprintln!("{:?}", e)
            // },
            // _ => {
                // return Err(Box::from(e))
            // }
        // }
    // }
// }
