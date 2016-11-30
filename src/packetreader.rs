extern crate byteorder;

use std::error;
use std::fmt;
use std::result;

use self::byteorder::{
    ByteOrder,
    BigEndian,
};


#[derive(Debug,PartialEq)]
pub enum Error {
    NotEnoughData,
    StringNotTerminated,
}


impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::NotEnoughData =>
                write!(f, "not enough data"),
            Error::StringNotTerminated =>
                write!(f, "string not terminated with null byte"),
        }
    }
}


impl error::Error for Error {
    fn description(&self) -> &str {
        "tftp packet read error"
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}


pub type Result<T> = result::Result<T, Error>;


#[derive(Debug)]
pub struct PacketReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> PacketReader<'a> {

    pub fn new(storage: &'a [u8]) -> PacketReader<'a> {
        PacketReader{
            buf: storage,
            pos: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn rem(&self) -> usize {
        self.buf.len() - self.pos
    }

    pub fn take_u16(&mut self) -> Result<u16> {
        if self.rem() >= 2 {
            let value = BigEndian::read_u16(&self.buf[self.pos..]);
            self.pos += 2;
            Ok(value)
        } else {
            Err(Error::NotEnoughData)
        }
    }

    pub fn take_string(&mut self) -> Result<String> {
        for pos in self.pos..self.buf.len() {
            if self.buf[pos] == 0u8 {
                let ref bytes = self.buf[self.pos..pos];
                // TODO: Convert from NetASCII to native.
                let string = String::from_utf8_lossy(bytes);
                self.pos = pos + 1;
                return Ok(string.into_owned())
            }
        }
        Err(Error::StringNotTerminated)
    }

    pub fn take_remaining(&mut self) -> Result<&'a [u8]> {
        let rem = &self.buf[self.pos..];
        self.pos = self.buf.len();
        Ok(rem)
    }
}


#[cfg(test)]
mod test {

    extern crate byteorder;

    use super::{Error, PacketReader};
    use self::byteorder::{
        ByteOrder,
        BigEndian,
    };

    #[test]
    fn test_create_new_buffer() {
        let mut storage = vec![0u8; 10];
        let buffer = PacketReader::new(&mut storage);
        assert_eq!(10, buffer.len());
        assert_eq!(0, buffer.pos());
        assert_eq!(10, buffer.rem());
    }

    #[test]
    fn test_take_u16() {
        let mut storage = vec![0u8; 2];
        BigEndian::write_u16(&mut storage, 1234);
        let mut buffer = PacketReader::new(&mut storage);
        assert_eq!(1234, buffer.take_u16().unwrap());
        assert_eq!(2, buffer.pos());
    }

    #[test]
    fn test_take_u16_out_of_range() {
        let mut storage = vec![0u8; 1];
        let mut buffer = PacketReader::new(&mut storage);
        assert_eq!(Error::NotEnoughData, buffer.take_u16().unwrap_err());
        assert_eq!(0, buffer.pos());
    }

    #[test]
    fn test_take_string() {
        let mut storage = "foobar\0".as_bytes();
        let mut buffer = PacketReader::new(&mut storage);
        assert_eq!("foobar", buffer.take_string().unwrap());
        assert_eq!(7, buffer.pos());
    }

    #[test]
    fn test_take_string_out_of_range() {
        let mut storage = vec!['a' as u8; 10];
        let mut buffer = PacketReader::new(&mut storage);
        assert_eq!(
            Error::StringNotTerminated,
            buffer.take_string().unwrap_err());
        assert_eq!(0, buffer.pos());
    }

}
