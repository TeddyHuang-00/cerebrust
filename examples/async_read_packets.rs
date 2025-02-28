use cerebrust::{comm::DataReader, device::DeviceConfig};

use std::time::Instant;

#[tokio::main]
async fn main() {
    // Using the default configurations
    let config = DeviceConfig::default();
    // Connect to the device (make sure the device is on and discoverable,
    // and NOT in BLE mode)
    let stream = config.connect().await.unwrap();
    // Create a data reader
    let mut data_reader = DataReader::new(stream);
    // Poll data packets asynchronously
    while let Ok(packet) = data_reader.poll_next().await {
        if let Some(eeg_power) = packet.eeg_power {
            println!("[{:?}]: {eeg_power:?}", Instant::now());
        }
    }
}
