use phf::{phf_map, Map};

use crate::devices::bluetooth::BluetoothDevice;
use crate::devices::*;

pub fn appraise(device: bluer::Device) -> Option<Box<dyn BluetoothDevice<Target = bluer::Device>>> {
    let address = device.address().to_string();

    BLUETOOTH_DEVICES.get(&address).map(|construct| construct(device))
}

type Constructor = fn(bluer::Device) -> Box<dyn BluetoothDevice<Target = bluer::Device>>;

static BLUETOOTH_DEVICES: Map<&'static str, Constructor> = phf_map! {
    "C8:0F:10:80:D0:AA" => miband::MiBand::<miband::OneS>::boxed
};
