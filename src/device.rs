//! Provides functionality to configure and connect to a NeuroSky device using
//! bluetooth. It includes a `DeviceConfig` struct for specifying the bluetooth
//! adapter, target device name, and RFCOMM channel, as well as methods for
//! discovering and connecting to the target device.
//!
//! # Examples
//!
//! ```rust
//! use cerebrust::device::DeviceConfig;
//!
//! #[tokio::main]
//! async fn main() -> bluer::Result<()> {
//!     let config = DeviceConfig::default()
//!         .with_adapter("hci0".to_string())
//!         .with_name("MyndBand".to_string())
//!         .with_channel(5);
//!
//!     let stream = config.connect().await?;
//!     // Use the stream to communicate with the device
//!
//!     Ok(())
//! }
//! ```
//!
//! # Errors
//!
//! Methods in this module may return errors related to Bluetooth session creation,
//! adapter retrieval, device discovery, and stream connection. These errors are
//! propagated as `bluer::Result` types.

use bluer::{
    Adapter, AdapterEvent, Address, Session,
    rfcomm::{SocketAddr, Stream},
};
use futures::{StreamExt, pin_mut};
use std::io;
use std::time::Duration;
use tokio::time::timeout;

/// Configuration for connecting to a NeuroSky device over Bluetooth.
#[derive(Debug)]
pub struct DeviceConfig {
    /// The name of the Bluetooth adapter to use.
    /// If not provided, the default adapter is used.
    pub adapter: Option<String>,
    /// The name of the target device. Default: "MyndBand".
    pub target_name: String,
    /// RFCOMM channel. Default: 5.
    pub channel: u8,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            adapter: None,
            target_name: "MyndBand".to_string(),
            channel: 5,
        }
    }
}

impl DeviceConfig {
    /// Updates the Bluetooth adapter name.
    ///
    /// # Arguments
    ///
    /// * `adapter` - The name of the Bluetooth adapter.
    ///
    /// # Returns
    ///
    /// * `Self` - The updated configuration.
    pub fn with_adapter(mut self, adapter: String) -> Self {
        self.adapter = Some(adapter);
        self
    }

    /// Updates the target device name.
    /// If not provided, the default name is "MyndBand".
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the target device.
    ///
    /// # Returns
    ///
    /// * `Self` - The updated configuration.
    pub fn with_name(mut self, name: String) -> Self {
        self.target_name = name;
        self
    }

    /// Updates the RFCOMM channel.
    /// If not provided, the default channel is 5.
    /// The channel is used to establish a connection to the target device.
    ///
    /// # Arguments
    ///
    /// * `channel` - The RFCOMM channel.
    ///
    /// # Returns
    ///
    /// * `Self` - The updated configuration.
    pub fn with_channel(mut self, channel: u8) -> Self {
        self.channel = channel;
        self
    }

    /// Gets the default Bluetooth adapter and powers it on.
    ///
    /// # Returns
    ///
    /// * `bluer::Result<Adapter>` - The default Bluetooth adapter.
    ///
    /// # Errors
    ///
    /// This function will return an error if the session creation, adapter retrieval,
    /// or powering on the adapter fails.
    pub async fn get_adapter(&self) -> bluer::Result<Adapter> {
        let session = Session::new().await?;
        let adapter = if let Some(name) = &self.adapter {
            session.adapter(name)?
        } else {
            session.default_adapter().await?
        };
        adapter.set_powered(true).await?;
        Ok(adapter)
    }

    /// Discovers the target Bluetooth device by name using the provided adapter.
    ///
    /// # Arguments
    ///
    /// * `adapter` - A reference to the Bluetooth adapter to use for discovery.
    /// * `name` - An optional name of the target device. If not provided, defaults to "MyndBand".
    ///
    /// # Returns
    ///
    /// * `bluer::Result<Address>` - The address of the discovered target device.
    ///
    /// # Errors
    ///
    /// This function will return an error if device discovery fails or if the device
    /// discovery times out.
    pub async fn try_find_device(&self, adapter: &Adapter) -> bluer::Result<Address> {
        let device_events = adapter.discover_devices().await?;
        pin_mut!(device_events);

        loop {
            match timeout(Duration::from_secs(1), device_events.next()).await {
                Ok(Some(AdapterEvent::DeviceAdded(addr))) => {
                    let device = adapter.device(addr)?;
                    match device.name().await? {
                        Some(name) if name == self.target_name => {
                            return Ok(addr);
                        }
                        _ => continue,
                    }
                }
                Ok(_) => continue,
                Err(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "Device discovery timed out",
                    )
                    .into());
                }
            }
        }
    }

    /// Builds an RFCOMM connection to the target device address.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address of the target device.
    ///
    /// # Returns
    ///
    /// * `bluer::Result<Stream>` - The RFCOMM stream connected to the target device.
    ///
    /// # Errors
    ///
    /// This function will return an error if the stream connection fails.
    pub async fn build_connection(&self, addr: Address) -> bluer::Result<Stream> {
        let target_sa = SocketAddr::new(addr, self.channel);
        let stream = Stream::connect(target_sa).await?;
        Ok(stream)
    }

    /// One-liner to get the default Bluetooth adapter, discover the target device,
    /// and build an RFCOMM connection to it.
    ///
    /// # Returns
    ///
    /// * `bluer::Result<Stream>` - The RFCOMM stream connected to the target device.
    ///
    /// # Errors
    ///
    /// This function will return an error if the default adapter retrieval, device
    /// discovery, or stream connection fails.
    pub async fn connect(&self) -> bluer::Result<Stream> {
        let adapter = self.get_adapter().await?;
        let addr = self.try_find_device(&adapter).await?;
        self.build_connection(addr).await
    }
}
