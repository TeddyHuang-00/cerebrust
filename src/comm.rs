//! Provides functionality to communicate with the NeuroSky device
//! using the RFCOMM protocol. It defines the `DataReader` struct which reads
//! data packets from the device and parses them into a `Packet` struct.
//!
//! # Enums
//!
//! - `Code`: Represents various data codes used in the NeuroSky device communication.
//!
//! # Structs
//!
//! - `Power`: Represents the EEG power spectrum values.
//! - `Packet`: Represents a data packet received from the NeuroSky device.
//! - `DataReader`: Reads and parses data packets from the bytes stream.
//!
//! # Example
//!
//! ```rust
//! use bluer::rfcomm::Stream;
//! use cerebrust::comm::DataReader;
//!
//! #[tokio::main]
//! async fn main() {
//!     let stream = ...; // Build RFCOMM stream
//!     let mut reader = DataReader::new(stream);
//!
//!     while let Some(packet) = reader.next() {
//!         println!("{:?}", packet);
//!     }
//! }
//! ```
//!
//! # Errors
//!
//! The `poll_next` method in `DataReader` returns an `Error` if there is an issue
//! reading from the stream or if the packet is corrupted.

use std::io::Error;

use bluer::rfcomm::Stream;
use futures::executor::block_on;
use tokio::io::AsyncReadExt;

/// Represents the different data codes used in the NeuroSky device communication.
/// Each code corresponds to a specific type of data that can be received from
/// the device. The codes are defined as per the NeuroSky protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Code {
    /// Single-byte u8
    PoorSignal = 0x02,
    /// Single-byte u8
    Attention = 0x04,
    /// Single-byte u8
    Meditation = 0x05,
    /// Multi-byte i16
    RawWave = 0x80,
    /// Multi-byte u24 * 8
    AsicEegPower = 0x83,
    /// Reserved
    Extended = 0x55,
    /// Sync byte
    Sync = 0xAA,
    /// Unknown code
    Unknown = 0xFF,
}

impl From<u8> for Code {
    fn from(value: u8) -> Self {
        match value {
            0x02 => Code::PoorSignal,
            0x04 => Code::Attention,
            0x05 => Code::Meditation,
            0x80 => Code::RawWave,
            0x83 => Code::AsicEegPower,
            0x55 => Code::Extended,
            0xAA => Code::Sync,
            _ => Code::Unknown,
        }
    }
}

/// Represents the EEG power spectrum values.
#[derive(Debug, Default)]
pub struct Power {
    /// Delta (0.5 ~ 2.75 Hz)
    pub delta: u32,
    /// Theta (3.5 ~ 7.75 Hz)
    pub theta: u32,
    /// Low alpha (7.5 ~ 9.25 Hz)
    pub low_alpha: u32,
    /// High alpha (9.5 ~ 11.75 Hz)
    pub high_alpha: u32,
    /// Low beta (12 ~ 14.75 Hz)
    pub low_beta: u32,
    /// High beta (15 ~ 21.75 Hz)
    pub high_beta: u32,
    /// Low gamma (22 ~ 30.75 Hz)
    pub low_gamma: u32,
    /// Mid gamma (31 ~ 100 Hz)
    pub mid_gamma: u32,
}

/// Represents a data packet received from the NeuroSky device.
#[derive(Debug, Default)]
pub struct Packet {
    /// Signal quality (0 ~ 255)
    pub poor_signal: Option<u8>,
    /// Attention eSense (0 ~ 100)
    pub attention: Option<u8>,
    /// Meditation eSense (0 ~ 100)
    pub meditation: Option<u8>,
    /// Raw wave value (-32768 ~ 32767)
    pub raw_wave: Option<i16>,
    /// EEG power spectrum values (uV^2)
    /// Delta (0.5 ~ 2.75 Hz)
    pub eeg_power: Option<Power>,
}

/// Represents the different types of packets that can be received from the
/// NeuroSky device. Each variant corresponds to a specific combination of data
/// that can be received.
#[derive(Debug)]
pub enum PacketVariant {
    /// Packet with only raw wave value @ 512Hz
    RawWave { raw_wave: i16 },
    /// Packet with eSense values and EEG power spectrum @ 1Hz
    EegPower {
        poor_signal: u8,
        attention: u8,
        meditation: u8,
        eeg_power: Power,
    },
}

impl TryInto<PacketVariant> for Packet {
    type Error = &'static str;

    /// Attempts to convert the `Packet` into a `PacketVariant`. If the packet
    /// contains both eSense values and EEG power spectrum, it returns the
    /// `EegPower` variant. If it contains only the raw wave value, it returns
    /// the `RawWave` variant. If neither is present, it returns an error.
    fn try_into(self) -> Result<PacketVariant, Self::Error> {
        if let (Some(poor_signal), Some(attention), Some(meditation), Some(eeg_power)) = (
            self.poor_signal,
            self.attention,
            self.meditation,
            self.eeg_power,
        ) {
            Ok(PacketVariant::EegPower {
                poor_signal,
                attention,
                meditation,
                eeg_power,
            })
        } else if let Some(raw_wave) = self.raw_wave {
            Ok(PacketVariant::RawWave { raw_wave })
        } else {
            Err("Invalid packet type")
        }
    }
}

/// Represents a data reader that reads and parses data packets from the
/// NeuroSky device.
pub struct DataReader {
    /// The RFCOMM stream to read data from
    stream: Stream,
}

impl DataReader {
    pub fn new(stream: Stream) -> DataReader {
        DataReader { stream }
    }

    /// Reads the next data packet from the stream. It synchronizes with the
    /// NeuroSky device, reads the packet length, and then reads the payload
    /// and checksum. It verifies the checksum and parses the payload into a
    /// `Packet` struct. If the packet is corrupted or invalid, it retries
    /// reading the packet until a valid one is received.
    pub async fn poll_next(&mut self) -> Result<Packet, Error> {
        let mut packet = Packet::default();
        loop {
            let mut sync = 0;
            while sync < 2 {
                // Sync with the NeuroSky device until two sync bytes are received
                if self.stream.read_u8().await? == Code::Sync as u8 {
                    sync += 1;
                } else {
                    sync = 0;
                }
            }
            let mut packet_length = self.stream.read_u8().await? as usize;
            while packet_length == Code::Sync as usize {
                // Re-read the packet length if it is another sync byte
                packet_length = self.stream.read_u8().await? as usize;
            }
            if packet_length > Code::Sync as usize {
                // Start-over if the packet length is invalid
                continue;
            }
            // Read the payload and checksum
            let mut payload = vec![0u8; packet_length as usize];
            self.stream.read_exact(&mut payload).await?;
            let checksum = self.stream.read_u8().await?;
            // Verify the checksum
            let calculated_checksum = 255 - payload.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
            if calculated_checksum != checksum {
                // Start-over if the packet is corrupted
                eprintln!(
                    "Checksum mismatch: 0b{:08b} (Expected) != 0b{:08b} (Got)",
                    checksum, calculated_checksum
                );
                continue;
            }
            // Parse the payload
            let mut i = 0..packet_length;
            while let Some(idx) = i.next() {
                match Code::from(payload[idx]) {
                    // Single-byte codes
                    Code::PoorSignal => packet.poor_signal = Some(payload[i.next().unwrap()]),
                    Code::Attention => packet.attention = Some(payload[i.next().unwrap()]),
                    Code::Meditation => packet.meditation = Some(payload[i.next().unwrap()]),

                    // Multi-byte codes
                    Code::RawWave => {
                        let value_length = payload[i.next().unwrap()];
                        if value_length != 2 {
                            // Something is wrong with the data, but we don't know what
                            eprintln!("Unexpected raw wave length {}", value_length);
                        }
                        packet.raw_wave = Some(i16::from_be_bytes([
                            payload[i.next().unwrap()],
                            payload[i.next().unwrap()],
                        ]));
                    }
                    Code::AsicEegPower => {
                        let value_length = payload[i.next().unwrap()];
                        if value_length != 24 {
                            // Something is wrong with the data, but we don't know what
                            eprintln!("Unexpected ASIC EEG power length {}", value_length);
                        }
                        let mut values = [0; 8];
                        values.iter_mut().for_each(|x| {
                            *x = u32::from_be_bytes([
                                0,
                                payload[i.next().unwrap()],
                                payload[i.next().unwrap()],
                                payload[i.next().unwrap()],
                            ]);
                        });
                        packet.eeg_power = Some(Power {
                            delta: values[0],
                            theta: values[1],
                            low_alpha: values[2],
                            high_alpha: values[3],
                            low_beta: values[4],
                            high_beta: values[5],
                            low_gamma: values[6],
                            mid_gamma: values[7],
                        });
                    }

                    // Reserved code
                    Code::Extended => {
                        // Extended code level is undefined
                        eprintln!("Extended code level is not defined");
                    }
                    Code::Sync => {
                        // Sync code encountered
                        eprintln!("Sync code encountered");
                    }
                    Code::Unknown => {
                        // Unknown code encountered
                        eprintln!("Unknown code at {}: 0x{}", idx, payload[idx]);
                    }
                }
            }
            return Ok(packet);
        }
    }
}

impl Iterator for DataReader {
    type Item = Packet;

    /// Polls the next packet from the stream. It blocks until a valid packet
    /// is received. If an error occurs while reading the packet, it stops
    /// the iterator.
    fn next(&mut self) -> Option<Self::Item> {
        match block_on(self.poll_next()) {
            Ok(packet) => Some(packet),
            Err(_) => None, // Stop the iterator if an error occurs
        }
    }
}
