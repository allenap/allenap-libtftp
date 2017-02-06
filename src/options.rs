use std::fmt::Display;
use std::result;
use std::str::FromStr;

use super::packet::{Error, Result};
use super::packetreader;
use super::packetwriter;


/// TFTP transfer options. Defined in RFC-2347.
#[derive(Debug)]
pub struct Options {
    /// Block size; 8-65464 inclusive. Defined in RFC-2348.
    pub blksize:    Option<u16>,
    /// Time-out; 1-255 seconds, inclusive. Defined in RFC-2349.
    pub timeout:    Option<u8>,
    /// Transfer size; 0 for query. Defined in RFC-2349.
    pub tsize:      Option<u64>,
    /// Window size; 1-65535. Defined in RFC-7440.
    pub windowsize: Option<u16>,
}


impl Options {

    pub fn new() -> Options {
        Options{
            blksize: None,
            timeout: None,
            tsize: None,
            windowsize: None,
        }
    }

    /// Is one or more of the options set?
    pub fn is_set(&self) -> bool {
        self.blksize.is_some() || self.timeout.is_some() ||
            self.tsize.is_some() || self.windowsize.is_some()
    }

    /// Read options from the given reader.
    pub fn read<'a>
        (reader: &mut packetreader::PacketReader<'a>)
         -> Result<Self>
    {
        match reader.take_remaining() {
            Ok(buffer) => match Self::parse(buffer) {
                Ok(options) => Ok(options),
                Err(error) => Err(Error::InvalidOptions(error)),
            },
            Err(error) => Err(Error::ReadError(error)),
        }
    }

    /// Write options to the given writer.
    pub fn write
        (self, writer: &mut packetwriter::PacketWriter)
        -> Result<()>
    {
        if let Some(blksize) = self.blksize {
            writer.put_string("blksize")?;
            writer.put_string(&blksize.to_string())?;
        };
        if let Some(timeout) = self.timeout {
            writer.put_string("timeout")?;
            writer.put_string(&timeout.to_string())?;
        };
        if let Some(tsize) = self.tsize {
            writer.put_string("tsize")?;
            writer.put_string(&tsize.to_string())?;
        };
        if let Some(windowsize) = self.windowsize {
            writer.put_string("windowsize")?;
            writer.put_string(&windowsize.to_string())?;
        };
        Ok(())
    }

    /// Parse options from the given buffer.
    ///
    /// Note that errors arising from this method are *strings*.
    pub fn parse<'a>(buf: &'a [u8]) -> result::Result<Self, String> {
        let mut container = Self::new();
        let mut options = OptionStringIter::new(buf);
        loop {
            match options.next() {
                OptionString::Terminated(option) => {
                    let option = &String::from_utf8_lossy(option);
                    match options.next() {
                        OptionString::Terminated(value) => {
                            let value = &String::from_utf8_lossy(value);
                            container.parse_option(option, value)?;
                        },
                        OptionString::Unterminated(value) => {
                            let value = &String::from_utf8_lossy(value);
                            return Err(format!(
                                "Option {} has unterminated value {}",
                                option, value));
                        },
                        OptionString::None => {
                            return Err(format!(
                                "Option {} has no corresponding value",
                                option));
                        },
                    };
                },
                OptionString::Unterminated(option) => {
                    let option = &String::from_utf8_lossy(option);
                    return Err(format!(
                        "Option {} is unterminated",
                        option));
                },
                OptionString::None => {
                    return Ok(container);
                },
            };
        };
    }

    fn parse_option
        (&mut self, option: &str, value: &str) -> result::Result<(), String>
    {
        match option.to_lowercase().as_ref() {
            "blksize" => self.blksize = Some(
                Options::parse_blksize(value)?),
            "timeout" => self.timeout = Some(
                Options::parse_timeout(value)?),
            "tsize" => self.tsize = Some(
                Options::parse_tsize(value)?),
            "windowsize" => self.windowsize = Some(
                Options::parse_windowsize(value)?),
            _ => {
                // Ignore, as advised in RFC-2347.
                // TODO: Record or log unrecognised options?
            },
        };
        Ok(())
    }

    fn parse_blksize(value: &str) -> result::Result<u16, String> {
        Options::parse_value("blksize", value)
    }

    fn parse_timeout(value: &str) -> result::Result<u8, String> {
        Options::parse_value("timeout", value)
    }

    fn parse_tsize(value: &str) -> result::Result<u64, String> {
        Options::parse_value("tsize", value)
    }

    fn parse_windowsize(value: &str) -> result::Result<u16, String> {
        Options::parse_value("windowsize", value)
    }

    fn parse_value<T: FromStr>
        (option: &str, value: &str) -> result::Result<T, String>
        where <T as FromStr>::Err: Display
    {
        match T::from_str(value) {
            Ok(value) => Ok(value),
            Err(error) => Err(format!(
                "Invalid {} value {:?}: {}", option, value, error))
        }
    }

}


#[cfg(test)]
mod test_options {

    use super::Options;

    #[test]
    fn test_creating_new_options() {
        let options = Options::new();
        assert_eq!(options.blksize, None);
        assert_eq!(options.timeout, None);
        assert_eq!(options.tsize, None);
        assert_eq!(options.windowsize, None);
    }

    #[test]
    fn test_parsing_blksize() {
        assert_eq!(Options::parse_blksize("123"), Ok(123u16));
        assert_eq!(
            Options::parse_blksize("foo"), Err(
                ("Invalid blksize value \"foo\": ".to_string() +
                 "invalid digit found in string")));
        assert_eq!(
            Options::parse_blksize("65536"), Err(
                ("Invalid blksize value \"65536\": ".to_string() +
                 "number too large to fit in target type")));
    }

    #[test]
    fn test_parsing_timeout() {
        assert_eq!(Options::parse_timeout("123"), Ok(123u8));
        assert_eq!(
            Options::parse_timeout("foo"), Err(
                ("Invalid timeout value \"foo\": ".to_string() +
                 "invalid digit found in string")));
        assert_eq!(
            Options::parse_timeout("256"), Err(
                ("Invalid timeout value \"256\": ".to_string() +
                 "number too large to fit in target type")));
    }

    #[test]
    fn test_parsing_tsize() {
        assert_eq!(Options::parse_tsize("123"), Ok(123u64));
        assert_eq!(
            Options::parse_tsize("foo"), Err(
                ("Invalid tsize value \"foo\": ".to_string() +
                 "invalid digit found in string")));
        assert_eq!(
            Options::parse_tsize("18446744073709551616"), Err(
                ("Invalid tsize value \"18446744073709551616\": ".to_string() +
                 "number too large to fit in target type")));
    }

    #[test]
    fn test_parsing_windowsize() {
        assert_eq!(Options::parse_windowsize("123"), Ok(123u16));
        assert_eq!(
            Options::parse_windowsize("foo"), Err(
                ("Invalid windowsize value \"foo\": ".to_string() +
                 "invalid digit found in string")));
        assert_eq!(
            Options::parse_windowsize("65536"), Err(
                ("Invalid windowsize value \"65536\": ".to_string() +
                 "number too large to fit in target type")));
    }

    #[test]
    fn test_parsing_options() {
        let buf = "blksize\067\0timeout\076\0tsize\098\0windowsize\0429\0".as_bytes();
        let options = Options::parse(buf).unwrap();
        assert_eq!(options.blksize, Some(67));
        assert_eq!(options.timeout, Some(76));
        assert_eq!(options.tsize, Some(98));
        assert_eq!(options.windowsize, Some(429));
    }

    #[test]
    fn test_parsing_empty_options() {
        let buf = "".as_bytes();
        let options = Options::parse(buf).unwrap();
        assert_eq!(options.blksize, None);
        assert_eq!(options.timeout, None);
        assert_eq!(options.tsize, None);
        assert_eq!(options.windowsize, None);
    }

    #[test]
    fn test_parsing_incorrectly_terminated_option_results_in_error() {
        let buf = "blksize".as_bytes();  // No trailing null byte.
        assert_eq!(
            Options::parse(buf).unwrap_err(),
            "Option blksize is unterminated");
    }

    #[test]
    fn test_parsing_incorrectly_terminated_value_results_in_error() {
        let buf = "blksize\067".as_bytes();  // No trailing null byte.
        assert_eq!(
            Options::parse(buf).unwrap_err(),
            "Option blksize has unterminated value 67");
    }

    #[test]
    fn test_parsing_option_without_value_results_in_error() {
        let buf = "foo\0".as_bytes();
        assert_eq!(
            Options::parse(buf).unwrap_err(),
            "Option foo has no corresponding value");
    }

    #[test]
    fn test_parsing_option_with_empty_value_results_in_error() {
        let buf = "blksize\0\0".as_bytes();
        assert_eq!(
            Options::parse(buf).unwrap_err(),
            "Invalid blksize value \"\": ".to_string() +
                "cannot parse integer from empty string");
    }

}


#[derive(Debug,PartialEq)]
enum OptionString<'a> {
    Terminated(&'a [u8]),
    Unterminated(&'a [u8]),
    None,
}


#[derive(Debug)]
struct OptionStringIter<'a> {
    buf: &'a [u8],
    pos: usize,
}


impl<'a> OptionStringIter<'a> {

    fn new(buf: &'a [u8]) -> OptionStringIter<'a> {
        OptionStringIter{buf: buf, pos: 0}
    }

    fn next(&mut self) -> OptionString<'a> {
        for index in self.pos..self.buf.len() {
            if self.buf[index] == 0u8 {
                let cstr = &self.buf[self.pos..index];
                self.pos = index + 1;
                return OptionString::Terminated(cstr);
            }
        }
        if self.buf.len() > self.pos {
            let cstr = &self.buf[self.pos..];
            self.pos = self.buf.len();
            return OptionString::Unterminated(cstr);
        }
        else {
            return OptionString::None;
        }
    }

}


#[cfg(test)]
mod test_option_string {

    use super::OptionString;
    use super::OptionStringIter;

    #[test]
    fn test_split() {
        let buf = "one\0two\0three".as_bytes();
        let mut iter = OptionStringIter::new(buf);
        assert_eq!(iter.next(), OptionString::Terminated("one".as_bytes()));
        assert_eq!(iter.next(), OptionString::Terminated("two".as_bytes()));
        assert_eq!(iter.next(), OptionString::Unterminated("three".as_bytes()));
        assert_eq!(iter.next(), OptionString::None);
    }

    #[test]
    fn test_split_unterminated() {
        let buf = "one".as_bytes();
        let mut iter = OptionStringIter::new(buf);
        assert_eq!(iter.next(), OptionString::Unterminated("one".as_bytes()));
        assert_eq!(iter.next(), OptionString::None);
    }

    #[test]
    fn test_split_with_empty() {
        let buf = "one\0\0three".as_bytes();
        let mut iter = OptionStringIter::new(buf);
        assert_eq!(iter.next(), OptionString::Terminated("one".as_bytes()));
        assert_eq!(iter.next(), OptionString::Terminated("".as_bytes()));
        assert_eq!(iter.next(), OptionString::Unterminated("three".as_bytes()));
        assert_eq!(iter.next(), OptionString::None);
    }

    #[test]
    fn test_split_empty() {
        let buf = "".as_bytes();
        let mut iter = OptionStringIter::new(buf);
        assert_eq!(iter.next(), OptionString::None);
    }

}
