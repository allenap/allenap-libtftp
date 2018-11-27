extern crate byteorder;

use std::error;
use std::fmt;
use std::io;
use std::result;

use super::options::Options;
use super::packetreader;
use super::packetwriter;


/// An error reading, parsing, or writing a packet.
#[derive(Debug,PartialEq)]
pub enum Error {
    /// The operation is invalid / not recognised.
    InvalidOpCode(u16),
    /// The transfer mode is invalid / not recognised.
    InvalidTransferMode(String),
    /// The error code is invalid / not recognised.
    InvalidErrorCode(u16),
    /// The options are invalid / not recognised.
    ///
    /// Okay, perhaps *not recognised* is not appropriate here, because
    /// the TFTP RFCs state that unrecognised options should be ignored.
    /// Only recognised options are ackowledged in an `OACK` packet,
    /// meaning that both sides know the options that the other side
    /// understands and does not understand before the transfer begins.
    InvalidOptions(String),
    /// A packet could not be read / deserialised.
    ReadError(packetreader::Error),
    /// A packet could not be written / serialised.
    WriteError(packetwriter::Error),
}


impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::InvalidOpCode(opcode) =>
                write!(f, "invalid operation: {}", opcode),
            Error::InvalidTransferMode(ref txmode) =>
                write!(f, "invalid transfer mode: {:?}", txmode),
            Error::InvalidErrorCode(errcode) =>
                write!(f, "invalid error code: {}", errcode),
            Error::InvalidOptions(ref options) =>
                write!(f, "invalid options: {:?}", options),
            Error::ReadError(ref error) =>
                write!(f, "packet could not be read: {:?}", error),
            Error::WriteError(ref error) =>
                write!(f, "packet could not be written: {:?}", error),
        }
    }
}


impl error::Error for Error {
    fn description(&self) -> &str {
        "tftp packet error"
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::ReadError(ref error) => Some(error),
            Error::WriteError(ref error) => Some(error),
            _ => None,
        }
    }
}


impl From<packetreader::Error> for Error {
    fn from(error: packetreader::Error) -> Error {
        Error::ReadError(error)
    }
}


impl From<packetwriter::Error> for Error {
    fn from(error: packetwriter::Error) -> Error {
        Error::WriteError(error)
    }
}


impl From<Error> for io::Error {
    fn from(error: Error) -> io::Error {
        io::Error::new(io::ErrorKind::Other, error)
    }
}


pub type Result<T> = result::Result<T, Error>;


/// The operation code that begins every TFTP packet.
#[derive(Debug)]
pub enum OpCode {
    /// Read request.
    RRQ = 1,
    /// Write request.
    WRQ = 2,
    /// Data.
    DATA = 3,
    /// Acknowledgment.
    ACK = 4,
    /// Error.
    ERROR = 5,
    /// Option Acknowledgment.
    OACK = 6,
}

impl OpCode {
    fn read(buffer: &mut packetreader::PacketReader) -> Result<Self> {
        let code = buffer.take_u16()?;
        match Self::from(code) {
            Some(opcode) => Ok(opcode),
            None => Err(Error::InvalidOpCode(code)),
        }
    }

    pub fn write(self, writer: &mut packetwriter::PacketWriter) -> Result<()> {
        writer.put_u16(self as u16)?;
        Ok(())
    }

    fn from(opcode: u16) -> Option<Self> {
        use self::OpCode::*;
        match opcode {
            1 => Some(RRQ),
            2 => Some(WRQ),
            3 => Some(DATA),
            4 => Some(ACK),
            5 => Some(ERROR),
            6 => Some(OACK),
            _ => None,
        }
    }
}


/// A filename as found in a `RRQ` or `WRQ` packet.
///
/// In TFTP, filenames are byte strings, transferred as NetASCII. There
/// is no implicit or explicit encoding, like UTF-8, but this library
/// makes a choice to parse and encode with UTF-8. This will be correct
/// for 7-bit ASCII, which likely reflects the intentions of TFTP's
/// creators, but may cause some incompatibility in real-world use,
/// hence this choice may be revisited.
///
/// NetASCII is an anachronistic fly in the ointment that this library
/// does not yet even attempt to support.
#[derive(Debug)]
pub struct Filename(pub String);

impl Filename {
    fn read(buffer: &mut packetreader::PacketReader) -> Result<Self> {
        Ok(Filename(buffer.take_string()?))
    }

    pub fn write(self, writer: &mut packetwriter::PacketWriter) -> Result<()> {
        writer.put_string(&self.0)?;
        Ok(())
    }
}


/// The transfer mode to use.
#[derive(Debug)]
pub enum TransferMode {
    /// NetASCII is obsolete and potentially **harmful** to your data.
    NetASCII,
    /// Octet means transfer bytes unaltered.
    Octet,
}

impl TransferMode {
    fn read(buffer: &mut packetreader::PacketReader) -> Result<Self> {
        let mode = buffer.take_string()?;
        match TransferMode::parse(&mode.as_bytes()) {
            Some(txmode) => Ok(txmode),
            None => Err(Error::InvalidTransferMode(mode)),
        }
    }

    pub fn write(self, writer: &mut packetwriter::PacketWriter) -> Result<()> {
        writer.put_string(match self {
            TransferMode::NetASCII => "netascii",
            TransferMode::Octet => "octet",
        })?;
        Ok(())
    }

    fn parse(buffer: &[u8]) -> Option<Self> {
        if buffer.eq_ignore_ascii_case("netascii".as_bytes()) {
            Some(TransferMode::NetASCII)
        }
        else if buffer.eq_ignore_ascii_case("octet".as_bytes()) {
            Some(TransferMode::Octet)
        }
        else {
            None
        }
    }
}


/// The block number in a `DATA` or `ACK` packet.
#[derive(Debug)]
pub struct BlockNum(pub u16);

impl BlockNum {
    fn read(buffer: &mut packetreader::PacketReader) -> Result<Self> {
        let blocknum = buffer.take_u16()?;
        Ok(BlockNum(blocknum))
    }

    pub fn write(self, writer: &mut packetwriter::PacketWriter) -> Result<()> {
        writer.put_u16(self.0)?;
        Ok(())
    }
}


/// The payload of a `DATA` packet.
#[derive(Debug)]
pub struct Data<'a>(pub &'a [u8]);

impl<'a> Data<'a> {
    fn read(buffer: &mut packetreader::PacketReader<'a>) -> Result<Self> {
        let data = buffer.take_remaining()?;
        Ok(Data(data))
    }

    pub fn write(self, writer: &mut packetwriter::PacketWriter) -> Result<()> {
        writer.put_bytes(&self.0)?;
        Ok(())
    }
}


/// The code in an `ERROR` packet.
///
/// Unless specified otherwise, these codes are all defined in RFC-1350.
#[derive(Debug)]
pub enum ErrorCode {
    /// Not defined, see error message (if any).
    NotDefined = 0,
    /// File not found.
    FileNotFound = 1,
    /// Access violation.
    AccessViolation = 2,
    /// Disk full or allocation exceeded.
    DiskFull = 3,
    /// Illegal TFTP operation.
    IllegalOperation = 4,
    /// Unknown transfer ID.
    UnknownTransferId = 5,
    /// File already exists.
    FileAlreadyExists = 6,
    /// No such user.
    NoSuchUser = 7,
    /// Options not acceptable. Defined in RFC-2347.
    BadOptions = 8,
}

impl ErrorCode {
    fn read(buffer: &mut packetreader::PacketReader) -> Result<Self> {
        let code = buffer.take_u16()?;
        match Self::from(code) {
            Some(errorcode) => Ok(errorcode),
            None => Err(Error::InvalidErrorCode(code)),
        }
    }

    pub fn write(self, writer: &mut packetwriter::PacketWriter) -> Result<()> {
        writer.put_u16(self as u16)?;
        Ok(())
    }

    fn from(code: u16) -> Option<Self> {
        use self::ErrorCode::*;
        match code {
            0 => Some(NotDefined),
            1 => Some(FileNotFound),
            2 => Some(AccessViolation),
            3 => Some(DiskFull),
            4 => Some(IllegalOperation),
            5 => Some(UnknownTransferId),
            6 => Some(FileAlreadyExists),
            7 => Some(NoSuchUser),
            8 => Some(BadOptions),
            _ => None,
        }
    }
}


/// The message in an `ERROR` packet.
#[derive(Debug)]
pub struct ErrorMessage(pub String);

impl ErrorMessage {
    fn read(buffer: &mut packetreader::PacketReader) -> Result<Self> {
        Ok(ErrorMessage(buffer.take_string()?))
    }

    pub fn write(self, writer: &mut packetwriter::PacketWriter) -> Result<()> {
        writer.put_string(&self.0)?;
        Ok(())
    }
}


/// A packet of the Trivial File Transfer Protocol.
#[derive(Debug)]
pub enum Packet<'a> {
    Read(Filename, TransferMode, Options),
    Write(Filename, TransferMode, Options),
    Data(BlockNum, Data<'a>),
    Ack(BlockNum),
    Error(ErrorCode, ErrorMessage),
    OAck(Options),
}

impl<'a> Packet<'a> {
    pub fn parse(buffer: &'a [u8]) -> Result<Self>
        where Self: 'a
    {
        let mut buffer = packetreader::PacketReader::new(&buffer);
        match OpCode::read(&mut buffer)? {
            OpCode::RRQ => Ok(Packet::Read(
                Filename::read(&mut buffer)?,
                TransferMode::read(&mut buffer)?,
                Options::read(&mut buffer)?,
            )),
            OpCode::WRQ => Ok(Packet::Write(
                Filename::read(&mut buffer)?,
                TransferMode::read(&mut buffer)?,
                Options::read(&mut buffer)?,
            )),
            OpCode::DATA => Ok(Packet::Data(
                BlockNum::read(&mut buffer)?,
                Data::read(&mut buffer)?,
            )),
            OpCode::ACK => Ok(Packet::Ack(
                BlockNum::read(&mut buffer)?,
            )),
            OpCode::ERROR => Ok(Packet::Error(
                ErrorCode::read(&mut buffer)?,
                ErrorMessage::read(&mut buffer)?,
            )),
            OpCode::OACK => Ok(Packet::OAck(
                Options::read(&mut buffer)?,
            )),
        }
    }

    pub fn opcode(&self) -> OpCode {
        match *self {
            Packet::Read(..) => OpCode::RRQ,
            Packet::Write(..) => OpCode::WRQ,
            Packet::Data(..) => OpCode::DATA,
            Packet::Ack(..) => OpCode::ACK,
            Packet::Error(..) => OpCode::ERROR,
            Packet::OAck(..) => OpCode::OACK,
        }
    }

    pub fn write(self, mut buffer: &'a mut [u8]) -> Result<usize> {
        let mut buffer = packetwriter::PacketWriter::new(&mut buffer);
        self.opcode().write(&mut buffer)?;
        match self {
            Packet::Read(filename, mode, options) => {
                filename.write(&mut buffer)?;
                mode.write(&mut buffer)?;
                options.write(&mut buffer)?;
            },
            Packet::Write(filename, mode, options) => {
                filename.write(&mut buffer)?;
                mode.write(&mut buffer)?;
                options.write(&mut buffer)?;
            },
            Packet::Data(block, data) => {
                block.write(&mut buffer)?;
                data.write(&mut buffer)?;
            },
            Packet::Ack(block) => {
                block.write(&mut buffer)?;
            },
            Packet::Error(code, message) => {
                code.write(&mut buffer)?;
                message.write(&mut buffer)?;
            },
            Packet::OAck(options) => {
                options.write(&mut buffer)?;
            },
        };
        Ok(buffer.pos())
    }
}
