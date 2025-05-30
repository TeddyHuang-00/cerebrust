use std::time::Instant;

use cerebrust::{DataReader, DeviceConfig, PacketVariant};

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
    let timer = Instant::now();
    while let Ok(packet) = data_reader.poll_next().await {
        // Optionally parse the packet into a specific variant
        // and handle it accordingly
        match packet.try_into() {
            Ok(PacketVariant::RawWave { .. }) => {}
            Ok(PacketVariant::EegPower {
                poor_signal,
                eeg_power,
                ..
            }) => {
                println!(
                    "[{:.02?}s]: {poor_signal:?} | {eeg_power:?}",
                    timer.elapsed().as_secs_f64()
                );
            }
            Err(e) => {
                eprintln!("Error parsing packet: {:?}", e);
                continue;
            }
        }
    }
}
