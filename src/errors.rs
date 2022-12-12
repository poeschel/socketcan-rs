// socketcan/src/errors.rs
//
// Implements errors for Rust SocketCAN library on Linux.
//
// This file is part of the Rust 'socketcan-rs' library.
//
// Licensed under the MIT license:
//   <LICENSE or http://opensource.org/licenses/MIT>
// This file may not be copied, modified, or distributed except according
// to those terms.

//! CAN bus errors.
//!
//!  information from https://raw.githubusercontent.com/torvalds/linux/master/
//!                 /include/uapi/linux/can/error.h

// TODO: Remove this before release
#![allow(missing_docs)]

use crate::Frame;
use std::{convert::TryFrom, error, fmt, io};

#[inline]
/// Helper function to retrieve a specific byte of frame data or returning an
/// `Err(..)` otherwise.
fn get_data(frame: &impl Frame, idx: u8) -> Result<u8, CanErrorDecodingFailure> {
    Ok(*frame
        .data()
        .get(idx as usize)
        .ok_or(CanErrorDecodingFailure::NotEnoughData(idx))?)
}

// ===== CanErrorDecodingFailure =====

/// Error decoding a CanError from a CanFrame.
#[derive(Copy, Clone, Debug)]
pub enum CanErrorDecodingFailure {
    /// The supplied CANFrame did not have the error bit set.
    NotAnError,
    /// The error type is not known and cannot be decoded.
    UnknownErrorType(u32),
    /// The error type indicated a need for additional information as `data`,
    /// but the `data` field was not long enough.
    NotEnoughData(u8),
    /// The error type `ControllerProblem` was indicated and additional
    /// information found, but not recognized.
    InvalidControllerProblem,
    /// The type of the ProtocolViolation was not valid
    InvalidViolationType,
    /// A location was specified for a ProtocolViolation, but the location
    /// was not valid.
    InvalidLocation,
    /// The supplied transciever error was invalid.
    InvalidTransceiverError,
}

impl error::Error for CanErrorDecodingFailure {}

impl fmt::Display for CanErrorDecodingFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use CanErrorDecodingFailure::*;
        let msg = match *self {
            NotAnError => "CAN frame is not an error",
            UnknownErrorType(_) => "unknown error type",
            NotEnoughData(_) => "not enough data",
            InvalidControllerProblem => "not a valid controller problem",
            InvalidViolationType => "not a valid violation type",
            InvalidLocation => "not a valid location",
            InvalidTransceiverError => "not a valid transceiver error",
        };
        write!(f, "{}", msg)
    }
}

// ===== CanError ====

#[derive(Copy, Clone, Debug)]
pub enum CanError {
    /// TX timeout (by netdevice driver)
    TransmitTimeout,
    /// Arbitration was lost. Contains the number after which arbitration was
    /// lost or 0 if unspecified
    LostArbitration(u8),
    /// Controller problem, see `ControllerProblem`
    ControllerProblem(ControllerProblem),
    /// Protocol violation at the specified `Location`. See `ProtocolViolation`
    /// for details.
    ProtocolViolation {
        vtype: ViolationType,
        location: Location,
    },
    /// Transceiver Error.
    TransceiverError,
    /// No ACK received for current CAN frame.
    NoAck,
    /// Bus off (due to too many detected errors)
    BusOff,
    /// Bus error (due to too many detected errors)
    BusError,
    /// The bus has been restarted
    Restarted,
    /// Unknown, possibly invalid, error
    Unknown(u32),
}

impl error::Error for CanError {}

impl fmt::Display for CanError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use CanError::*;
        match *self {
            TransmitTimeout => write!(f, "transmission timeout"),
            LostArbitration(n) => write!(f, "arbitration lost after {} bits", n),
            ControllerProblem(e) => write!(f, "controller problem: {}", e),
            ProtocolViolation { vtype, location } => {
                write!(f, "protocol violation at {}: {}", location, vtype)
            }
            TransceiverError => write!(f, "transceiver error"),
            NoAck => write!(f, "no ack"),
            BusOff => write!(f, "bus off"),
            BusError => write!(f, "bus error"),
            Restarted => write!(f, "restarted"),
            Unknown(errno) => write!(f, "unknown error ({})", errno),
        }
    }
}

impl embedded_can::Error for CanError {
    fn kind(&self) -> embedded_can::ErrorKind {
        match *self {
            CanError::ControllerProblem(cp) => match cp {
                ControllerProblem::ReceiveBufferOverflow
                | ControllerProblem::TransmitBufferOverflow => embedded_can::ErrorKind::Overrun,
                _ => embedded_can::ErrorKind::Other,
            },
            CanError::NoAck => embedded_can::ErrorKind::Acknowledge,
            _ => embedded_can::ErrorKind::Other,
        }
    }
}

// ===== ControllerProblem =====

#[derive(Copy, Clone, Debug)]
pub enum ControllerProblem {
    /// unspecified
    Unspecified,
    /// RX buffer overflow
    ReceiveBufferOverflow,
    /// TX buffer overflow
    TransmitBufferOverflow,
    /// reached warning level for RX errors
    ReceiveErrorWarning,
    /// reached warning level for TX errors
    TransmitErrorWarning,
    /// reached error passive status RX
    ReceiveErrorPassive,
    /// reached error passive status TX
    TransmitErrorPassive,
    /// recovered to error active state
    Active,
}

impl error::Error for ControllerProblem {}

impl fmt::Display for ControllerProblem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ControllerProblem::*;
        let msg = match *self {
            Unspecified => "unspecified controller problem",
            ReceiveBufferOverflow => "receive buffer overflow",
            TransmitBufferOverflow => "transmit buffer overflow",
            ReceiveErrorWarning => "ERROR WARNING (receive)",
            TransmitErrorWarning => "ERROR WARNING (transmit)",
            ReceiveErrorPassive => "ERROR PASSIVE (receive)",
            TransmitErrorPassive => "ERROR PASSIVE (transmit)",
            Active => "ERROR ACTIVE",
        };
        write!(f, "{}", msg)
    }
}

impl TryFrom<u8> for ControllerProblem {
    type Error = CanErrorDecodingFailure;

    fn try_from(val: u8) -> Result<Self, Self::Error> {
        use ControllerProblem::*;
        Ok(match val {
            0x00 => Unspecified,
            0x01 => ReceiveBufferOverflow,
            0x02 => TransmitBufferOverflow,
            0x04 => ReceiveErrorWarning,
            0x08 => TransmitErrorWarning,
            0x10 => ReceiveErrorPassive,
            0x20 => TransmitErrorPassive,
            0x40 => Active,
            _ => return Err(CanErrorDecodingFailure::InvalidControllerProblem),
        })
    }
}

// ===== ViolationType =====

#[derive(Copy, Clone, Debug)]
pub enum ViolationType {
    /// Unspecified Violation
    Unspecified,
    /// Single Bit Error
    SingleBitError,
    /// Frame formatting error
    FrameFormatError,
    /// Bit stuffing error
    BitStuffingError,
    /// A dominant bit was sent, but not received
    UnableToSendDominantBit,
    /// A recessive bit was sent, but not received
    UnableToSendRecessiveBit,
    /// Bus overloaded
    BusOverload,
    /// Bus is active (again)
    Active,
    /// Transmission Error
    TransmissionError,
}

impl error::Error for ViolationType {}

impl fmt::Display for ViolationType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ViolationType::*;
        let msg = match *self {
            Unspecified => "unspecified",
            SingleBitError => "single bit error",
            FrameFormatError => "frame format error",
            BitStuffingError => "bit stuffing error",
            UnableToSendDominantBit => "unable to send dominant bit",
            UnableToSendRecessiveBit => "unable to send recessive bit",
            BusOverload => "bus overload",
            Active => "active",
            TransmissionError => "transmission error",
        };
        write!(f, "{}", msg)
    }
}

impl TryFrom<u8> for ViolationType {
    type Error = CanErrorDecodingFailure;

    fn try_from(val: u8) -> Result<Self, Self::Error> {
        use ViolationType::*;
        Ok(match val {
            0x00 => Unspecified,
            0x01 => SingleBitError,
            0x02 => FrameFormatError,
            0x04 => BitStuffingError,
            0x08 => UnableToSendDominantBit,
            0x10 => UnableToSendRecessiveBit,
            0x20 => BusOverload,
            0x40 => Active,
            0x80 => TransmissionError,
            _ => return Err(CanErrorDecodingFailure::InvalidViolationType),
        })
    }
}

/// Location
///
/// Describes where inside a received frame an error occured.
#[derive(Copy, Clone, Debug)]
pub enum Location {
    /// Unspecified
    Unspecified,
    /// Start of frame.
    StartOfFrame,
    /// ID bits 28-21 (SFF: 10-3)
    Id2821,
    /// ID bits 20-18 (SFF: 2-0)
    Id2018,
    /// substitute RTR (SFF: RTR)
    SubstituteRtr,
    /// extension of identifier
    IdentifierExtension,
    /// ID bits 17-13
    Id1713,
    /// ID bits 12-5
    Id1205,
    /// ID bits 4-0
    Id0400,
    /// RTR bit
    Rtr,
    /// Reserved bit 1
    Reserved1,
    /// Reserved bit 0
    Reserved0,
    /// Data length
    DataLengthCode,
    /// Data section
    DataSection,
    /// CRC sequence
    CrcSequence,
    /// CRC delimiter
    CrcDelimiter,
    /// ACK slot
    AckSlot,
    /// ACK delimiter
    AckDelimiter,
    /// End-of-frame
    EndOfFrame,
    /// Intermission (between frames)
    Intermission,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Location::*;
        let msg = match *self {
            Unspecified => "unspecified location",
            StartOfFrame => "start of frame",
            Id2821 => "ID, bits 28-21",
            Id2018 => "ID, bits 20-18",
            SubstituteRtr => "substitute RTR bit",
            IdentifierExtension => "ID, extension",
            Id1713 => "ID, bits 17-13",
            Id1205 => "ID, bits 12-05",
            Id0400 => "ID, bits 04-00",
            Rtr => "RTR bit",
            Reserved1 => "reserved bit 1",
            Reserved0 => "reserved bit 0",
            DataLengthCode => "data length code",
            DataSection => "data section",
            CrcSequence => "CRC sequence",
            CrcDelimiter => "CRC delimiter",
            AckSlot => "ACK slot",
            AckDelimiter => "ACK delimiter",
            EndOfFrame => "end of frame",
            Intermission => "intermission",
        };
        write!(f, "{}", msg)
    }
}
impl TryFrom<u8> for Location {
    type Error = CanErrorDecodingFailure;

    fn try_from(val: u8) -> Result<Self, Self::Error> {
        Ok(match val {
            0x00 => Location::Unspecified,
            0x03 => Location::StartOfFrame,
            0x02 => Location::Id2821,
            0x06 => Location::Id2018,
            0x04 => Location::SubstituteRtr,
            0x05 => Location::IdentifierExtension,
            0x07 => Location::Id1713,
            0x0F => Location::Id1205,
            0x0E => Location::Id0400,
            0x0C => Location::Rtr,
            0x0D => Location::Reserved1,
            0x09 => Location::Reserved0,
            0x0B => Location::DataLengthCode,
            0x0A => Location::DataSection,
            0x08 => Location::CrcSequence,
            0x18 => Location::CrcDelimiter,
            0x19 => Location::AckSlot,
            0x1B => Location::AckDelimiter,
            0x1A => Location::EndOfFrame,
            0x12 => Location::Intermission,
            _ => return Err(CanErrorDecodingFailure::InvalidLocation),
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub enum TransceiverError {
    Unspecified,
    CanHighNoWire,
    CanHighShortToBat,
    CanHighShortToVcc,
    CanHighShortToGnd,
    CanLowNoWire,
    CanLowShortToBat,
    CanLowShortToVcc,
    CanLowShortToGnd,
    CanLowShortToCanHigh,
}

impl TryFrom<u8> for TransceiverError {
    type Error = CanErrorDecodingFailure;

    fn try_from(val: u8) -> Result<Self, Self::Error> {
        use TransceiverError::*;
        Ok(match val {
            0x00 => Unspecified,
            0x04 => CanHighNoWire,
            0x05 => CanHighShortToBat,
            0x06 => CanHighShortToVcc,
            0x07 => CanHighShortToGnd,
            0x40 => CanLowNoWire,
            0x50 => CanLowShortToBat,
            0x60 => CanLowShortToVcc,
            0x70 => CanLowShortToGnd,
            0x80 => CanLowShortToCanHigh,
            _ => return Err(CanErrorDecodingFailure::InvalidTransceiverError),
        })
    }
}

impl CanError {
    pub fn from_frame(frame: &impl Frame) -> Result<Self, CanErrorDecodingFailure> {
        if !frame.is_error() {
            return Err(CanErrorDecodingFailure::NotAnError);
        }

        match frame.err() {
            0x00000001 => Ok(CanError::TransmitTimeout),
            0x00000002 => Ok(CanError::LostArbitration(get_data(frame, 0)?)),
            0x00000004 => Ok(CanError::ControllerProblem(ControllerProblem::try_from(
                get_data(frame, 1)?,
            )?)),

            0x00000008 => Ok(CanError::ProtocolViolation {
                vtype: ViolationType::try_from(get_data(frame, 2)?)?,
                location: Location::try_from(get_data(frame, 3)?)?,
            }),

            0x00000010 => Ok(CanError::TransceiverError),
            0x00000020 => Ok(CanError::NoAck),
            0x00000040 => Ok(CanError::BusOff),
            0x00000080 => Ok(CanError::BusError),
            0x00000100 => Ok(CanError::Restarted),
            e => Err(CanErrorDecodingFailure::UnknownErrorType(e)),
        }
    }
}

pub trait ControllerSpecificErrorInformation {
    fn get_ctrl_err(&self) -> Option<&[u8]>;
}

impl<T: Frame> ControllerSpecificErrorInformation for T {
    #[inline]
    fn get_ctrl_err(&self) -> Option<&[u8]> {
        let data = self.data();

        if data.len() != 8 {
            None
        } else {
            Some(&data[5..])
        }
    }
}

#[derive(Debug)]
/// Errors opening socket
pub enum CanSocketOpenError {
    /// Device could not be found
    LookupError(nix::Error),

    /// System error while trying to look up device name
    IOError(io::Error),
}

impl error::Error for CanSocketOpenError {}

impl fmt::Display for CanSocketOpenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use CanSocketOpenError::*;
        match *self {
            LookupError(ref e) => write!(f, "CAN Device not found: {}", e),
            IOError(ref e) => write!(f, "IO: {}", e),
        }
    }
}

impl From<nix::Error> for CanSocketOpenError {
    fn from(e: nix::Error) -> CanSocketOpenError {
        CanSocketOpenError::LookupError(e)
    }
}

impl From<io::Error> for CanSocketOpenError {
    fn from(e: io::Error) -> CanSocketOpenError {
        CanSocketOpenError::IOError(e)
    }
}

// ===== ConstructionError =====

#[derive(Debug, Copy, Clone)]
/// Error that occurs when creating CAN packets
pub enum ConstructionError {
    /// CAN ID was outside the range of valid IDs
    IDTooLarge,
    /// Larger payload reported than can be held in the frame.
    TooMuchData,
}

impl error::Error for ConstructionError {}

impl fmt::Display for ConstructionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ConstructionError::*;
        match *self {
            IDTooLarge => write!(f, "CAN ID too large"),
            TooMuchData => {
                write!(f, "Payload is larger than CAN maximum of 8 bytes")
            }
        }
    }
}

