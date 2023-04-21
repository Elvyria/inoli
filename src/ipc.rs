use crate::{Error, devices::capabilities::battery::BatteryInfo, command::{self, Command}};
use std::{path::Path, sync::Arc, pin::Pin, io::{self, Cursor}};

use bluer::Address;
use futures::{Stream, StreamExt, lock::Mutex, stream::SelectAll};
use log::{debug, warn};
use tokio::{net::{UnixListener, UnixStream}, sync::{watch, mpsc}};

use crate::devices::Version;

type Messenger = Pin<Box<dyn Stream<Item = Message> + Send>>;

pub struct Ipc {
    listener:     Arc<UnixListener>,
    messengers:   Arc<Mutex<SelectAll<Messenger>>>,
    tx:           watch::Sender<Message>,
    rx:           watch::Receiver<Message>,

    commander:    mpsc::Sender<Command>,
    pub commands: Mutex<mpsc::Receiver<Command>>,
}

// enum ConnectionState {
    // Connected,
    // Disconnected,
// }

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Battery(u8),
    Heartrate(u8),
    Steps(u32),
}

impl From<BatteryInfo> for Message {
    fn from(info: BatteryInfo) -> Message { Message::Battery(info.level) }
}

impl Message {
    fn id(&self) -> u8 {
        match self {
            Message::Battery(_)   => 11,
            Message::Heartrate(_) => 12,
            Message::Steps(_)     => 13,
        }
    }

    pub fn to_le_bytes(self) -> Vec<u8> {
        let mut vec = vec![b'M', b'S', b'G', self.id()];

        match self {
            Message::Battery(v) | Message::Heartrate(v) => {
                vec.push(v);
            },
            Message::Steps(v) => {
                vec.extend_from_slice(&v.to_le_bytes());
            }
        }

        vec
    }
}

struct Info {
    name:     String,
    address:  Address,
    firmware: Version,
}

impl Ipc {
    pub fn new<P>(path: P) -> Result<Ipc, Error> 
        where
        P: AsRef<Path>
    {
        let listener = Arc::new(UnixListener::bind(path)?);
        let (tx, rx) = watch::channel(Message::Steps(0));
        let (commander, commands) = mpsc::channel(8);

        Ok(Self { listener, tx, rx, messengers: Arc::new(Mutex::new(SelectAll::new())), commander, commands: Mutex::new(commands) })
    }

    pub fn add_messenger<M>(&self, messenger: M)
        where
        M: Stream<Item = Message> + Send + 'static
    {
        let mut messengers = self.messengers.try_lock().expect("locking messengers for modification");

        messengers.push(Box::pin(messenger));
    }

    pub async fn transmit(&self) -> Result<(), Error> {
        let mut messengers = self.messengers.try_lock().expect("locking messsengers to pull and transmit");

        while let Some(message) = messengers.next().await {
            debug!("Transmitting message: {:?}", message.to_le_bytes());

            self.tx.send(message).unwrap();
        }

        Ok(())
    }

    pub async fn listen(&self) -> Result<(), std::io::Error> {
        loop {
            match self.listener.accept().await {
                Ok((stream, _)) => {
                    debug!("A new client has been connected");

                    let rx = self.rx.clone();
                    let commander = self.commander.clone();

                    tokio::spawn(async move {
                        Self::handle_client(stream, rx, commander).await.unwrap();
                    });
                },
                Err(e) => {
                    println!("couldn't accept client connection: {e}");
                    return Err(e)
                }
            }
        }
    }

    async fn handle_client(stream: UnixStream, mut message: watch::Receiver<Message>, commander: mpsc::Sender<Command>) -> Result<(), Error> {
        let mut buf = [0; 32];

        loop {
            tokio::select! {
                Ok(_) = message.changed() => {
                    stream.writable().await?;

                    Self::send(&stream, *message.borrow())?;
                }
                Ok(_) = stream.readable() => {
                    Self::read_and_command(&commander, &stream, &mut buf)?;
                }
                else => return Ok(())
            }
        }
    }

    fn read_and_command(commander: &mpsc::Sender<Command>, stream: &UnixStream, mut buf: &mut [u8]) -> Result<(), std::io::Error> {
        match stream.try_read(buf) {
            Ok(n) => {
                if n == 0 { return Err(io::Error::new(io::ErrorKind::BrokenPipe, "read 0 bytes from stream, client has disconnected")) }

                debug!("Recieved message {:?}", &buf);

                use command::MAGIC;

                while let Some(i) = buf.windows(MAGIC.len()).position(|window| window == MAGIC) {

                    let mut r = Cursor::new(&buf[MAGIC.len() + i..]);

                    match Command::read(&mut r) {
                        Ok(command) => {
                            commander.try_send(command).unwrap()
                        }
                        Err(e) => warn!("Couldn't parse incoming message: {e}")
                    }

                    let len = (i as u64 + r.position()).min(buf.len() as u64);
                    buf = &mut buf[(len as usize)..]
                }

                Ok(())
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Ok(()),
            Err(e) => Err(e)
        }
    }

    fn send(stream: &UnixStream, message: Message) -> Result<(), std::io::Error> {
        debug!("Sending message {:?}", message);

        let buf = message.to_le_bytes();

        match stream.try_write(&buf) {
            Ok(n) => {
                if n == 0 { return Err(io::Error::new(io::ErrorKind::BrokenPipe, "couldn't write a single byte to stream, client has disconnected")) }

                if n != buf.len() { warn!("Couldn't write all bytes in message, client might be confused. This incident will be ignored...") }

                Ok(())
            },
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Ok(()),
            Err(e) => Err(e)
        }
    }
}
