use cerebrust::{comm::DataReader, device::DeviceConfig};

use std::time::Instant;

use futures::executor::block_on;

fn main() {
    // Using the default configurations
    let config = DeviceConfig::default();
    // Connect to the device (make sure the device is on and discoverable,
    // and NOT in BLE mode)
    let stream = block_on(config.connect()).unwrap();
    // Create a data reader
    let data_reader = DataReader::new(stream);
    // Synchronously read data packets
    for packet in data_reader {
        if let Some(eeg_power) = packet.eeg_power {
            println!("[{:?}]: {eeg_power:?}", Instant::now());
        }
    }
}
