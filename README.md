# cerebrust

A library for interfacing with NeuroSky devices over Bluetooth, using Rust.

> [!IMPORTANT]
> Due to limitations in the availability of `bluez`, this library is only compatible with Linux systems.

## Features

- Connect to NeuroSky devices via RFCOMM.
- Parse data packets, including raw values, signal quality, attention, meditation, and EEG power values.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
cerebrust = "0.1.0"
```

Create a `DeviceConfig` and connect to the device:

```rust
use cerebrust::device::DeviceConfig;

#[tokio::main]
async fn main() -> bluer::Result<()> {
    let stream = DeviceConfig::default()
        .with_adapter("hci0".to_string())
        .with_name("MyndBand".to_string())
        .with_channel(5)
        .connect()
        .await
        .unwrap();
    // Use the `stream` to read data
    Ok(())
}
```

Read packets with `DataReader`:

```rust
use cerebrust::comm::DataReader;
//...
#[tokio::main]
async fn main() {
    // ...
    let mut data_reader = DataReader::new(stream);
    while let Ok(packet) = data_reader.poll_next().await {
        if let Some(eeg_power) = packet.eeg_power {
            println!("{eeg_power:?}");
        }
    }
}
```

See the [examples](./examples) for full usage (requires a NeuroSky device).

## License

Licensed under the [MIT license](./LICENSE).
