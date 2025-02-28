//! cerebrust is a library for interfacing with NeuroSky devices over Bluetooth.
//! It provides functionality to configure and connect to a NeuroSky device using
//! Bluetooth, as well as to read and parse data packets from the data stream.

pub mod comm;
pub mod device;

#[cfg(test)]
mod tests {
    use super::*;

    /// Doing an all-in-one test to build an RFCOMM stream, read a packet, and
    /// parse it. Unit testing each function is omitted intentionally, as it
    /// may cause data races.
    ///
    /// NOTE: This test requires a NeuroSky device to be connected to the
    /// Bluetooth adapter. The device name is assumed to be "MyndBand".
    #[tokio::test]
    async fn test_build_stream_and_parse() {
        let config = device::DeviceConfig::default()
            .with_adapter("hci0".to_string())
            .with_name("MyndBand".to_string())
            .with_channel(5);
        let stream = config
            .connect()
            .await
            .expect("Failed to build RFCOMM stream");
        println!("Local address: {:?}", stream.as_ref().local_addr().unwrap());
        println!("Remote address: {:?}", stream.peer_addr().unwrap());
        println!("Security: {:?}", stream.as_ref().security().unwrap());
        let mut data_reader = comm::DataReader::new(stream);
        let packet = data_reader
            .poll_next()
            .await
            .expect("Failed to poll next packet");
        println!("{:?}", packet);
    }
}
