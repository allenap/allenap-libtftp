extern crate byteorder;

use std::ascii::AsciiExt;
use std::error;
use std::fmt;
use std::result;

use self::byteorder::{
    ByteOrder,
    BigEndian,
};


#[derive(Debug,PartialEq)]
pub enum Error {
    NotEnoughSpace,
    StringNotASCII,
    StringContainsNull,
}


impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::NotEnoughSpace =>
                write!(f, "not enough space for packet data"),
            Error::StringNotASCII =>
                write!(f, "string is not ASCII"),
            Error::StringContainsNull =>
                write!(f, "string contains null byte"),
        }
    }
}


impl error::Error for Error {
    fn description(&self) -> &str {
        "tftp packet write error"
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}


pub type Result<T> = result::Result<T, Error>;


#[derive(Debug)]
pub struct PacketWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> PacketWriter<'a> {

    pub fn new(storage: &'a mut [u8]) -> Self {
        PacketWriter{
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

    pub fn put_u16(&mut self, value: u16) -> Result<()> {
        if self.rem() >= 2 {
            BigEndian::write_u16(&mut self.buf[self.pos..], value);
            self.pos += 2;
            Ok(())
        } else {
            Err(Error::NotEnoughSpace)
        }
    }

    pub fn put_string(&mut self, value: &str) -> Result<()> {
        if value.is_ascii() {
            if value.contains("\0") {
                Err(Error::StringContainsNull)
            }
            else {
                let end = self.pos + value.len();
                // Greater-than-or-equals because of the null terminator.
                if end >= self.buf.len() {
                    Err(Error::NotEnoughSpace)
                } else {
                    // TODO: NetASCII nonsense.
                    self.buf[self.pos..end].copy_from_slice(value.as_bytes());
                    self.buf[end] = 0u8;
                    self.pos = end + 1;
                    Ok(())
                }
            }
        } else {
            Err(Error::StringNotASCII)
        }
    }

    pub fn put_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        let end = self.pos + bytes.len();
        if end > self.buf.len() {
            Err(Error::NotEnoughSpace)
        } else {
            self.buf[self.pos..end].copy_from_slice(bytes);
            self.pos = end;
            Ok(())
        }
    }

    pub fn get(mut self) -> (&'a mut [u8], usize) {
        (self.buf, self.pos)
    }
}


#[cfg(test)]
mod test {

    use super::{Error, PacketWriter};

    #[test]
    fn test_create_new_buffer() {
        let mut storage = vec![0u8; 10];
        let buffer = PacketWriter::new(&mut storage);
        assert_eq!(10, buffer.len());
        assert_eq!(0, buffer.pos());
        assert_eq!(10, buffer.rem());
    }

    #[test]
    fn test_get_underlying_storage() {
        let mut storage = vec![0u8; 10];
        let buffer = PacketWriter::new(&mut storage);
        let (storage, position) = buffer.get();
        assert_eq!(10, storage.len());
        assert_eq!(0, position);
    }

    #[test]
    fn test_put_u16() {
        let mut storage = vec![0u8; 3];
        let mut buffer = PacketWriter::new(&mut storage);
        buffer.put_u16(1234).unwrap();
        assert_eq!(2, buffer.pos());
        assert_eq!(
            (&mut [4u8, 210, 0][..], 2),
            buffer.get());
    }

    #[test]
    fn test_put_u16_out_of_range() {
        let mut storage = vec![0u8; 1];
        let mut buffer = PacketWriter::new(&mut storage);
        assert_eq!(Error::NotEnoughSpace, buffer.put_u16(1).unwrap_err());
        assert_eq!(0, buffer.pos());
    }

    #[test]
    fn test_put_string() {
        let mut storage = vec![0u8; 5];
        let mut buffer = PacketWriter::new(&mut storage);
        buffer.put_string("foo").unwrap();
        assert_eq!(4, buffer.pos());
        assert_eq!(
            (&mut [102u8, 111, 111, 0, 0][..], 4),
            buffer.get());
    }

    #[test]
    fn test_put_string_out_of_range() {
        let mut storage = vec![0u8; 6];
        let mut buffer = PacketWriter::new(&mut storage);
        assert_eq!(
            Error::NotEnoughSpace,
            buffer.put_string("foobar").unwrap_err());
        assert_eq!(0, buffer.pos());
    }

    #[test]
    fn test_put_string_not_ascii() {
        let mut storage = vec![0u8; 6];
        let mut buffer = PacketWriter::new(&mut storage);
        assert_eq!(
            Error::StringNotASCII,
            buffer.put_string("…").unwrap_err());
        assert_eq!(0, buffer.pos());
    }

    #[test]
    fn test_put_string_with_null() {
        let mut storage = vec![0u8; 6];
        let mut buffer = PacketWriter::new(&mut storage);
        assert_eq!(
            Error::StringContainsNull,
            buffer.put_string("foo\0bar").unwrap_err());
        assert_eq!(0, buffer.pos());
    }

}
