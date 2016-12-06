#[macro_use]
extern crate slog;

use std::io;
use std::net;
use std::error::Error;

pub mod options;
pub mod packet;
mod packetreader;
mod packetwriter;
pub mod rrq;

use self::options::Options;
use self::packet::{Filename, Packet, TransferMode};


pub fn serve(
    addr: net::SocketAddr, handler: &Handler, logger: &slog::Logger)
    -> io::Result<()>
{
    let socket = try!(net::UdpSocket::bind(addr));
    info!(logger, "Listening"; "address" => format!("{}", addr));

    // RFC-2347 says "The maximum size of a request packet is 512 octets."
    let mut bufin = [0; 512];
    let mut bufout = [0; 4 + 512];
    loop {
        match socket.recv_from(&mut bufin) {
            Ok((size, src)) => {
                match Packet::parse(&mut bufin[..size]) {
                    Ok(packet) => {
                        match handler.handle(addr, src, packet) {
                            Some(packet) => {
                                let size = packet.write(&mut bufout)?;
                                socket.send_to(&bufout[..size], &src)?;
                            },
                            None => {},
                        };
                    },
                    Err(error) => warn!(
                        logger, "Ignoring malformed packet";
                        "error" => error.description()),
                }
            },
            Err(error) => return Err(error),
        }
    };
}


pub trait Handler {

    fn handle(
        &self, local: net::SocketAddr, remote: net::SocketAddr, packet: Packet)
        -> Option<Packet>
    {
        match packet {
            Packet::Read(filename, txmode, options) =>
                self.handle_rrq(local, remote, filename, txmode, options),
            Packet::Write(filename, txmode, options) =>
                self.handle_wrq(local, remote, filename, txmode, options),
            packet =>
                self.handle_other(local, remote, packet),
        }
    }

    fn handle_rrq(
        &self, _local: net::SocketAddr, _remote: net::SocketAddr,
        _filename: Filename, _txmode: TransferMode, _options: Options)
        -> Option<Packet>
    {
        Some(Packet::Error(
            packet::ErrorCode::AccessViolation,
            packet::ErrorMessage("read not supported".to_owned()),
        ))
    }

    fn handle_wrq(
        &self, _local: net::SocketAddr, _remote: net::SocketAddr,
        _filename: Filename, _txmode: TransferMode, _options: Options)
        -> Option<Packet>
    {
        Some(Packet::Error(
            packet::ErrorCode::AccessViolation,
            packet::ErrorMessage("write not supported".to_owned()),
        ))
    }

    fn handle_other(
        &self, _local: net::SocketAddr, _remote: net::SocketAddr,
        _packet: Packet)
        -> Option<Packet>
    {
        None  // Ignore.
    }

}


fn make_socket(peer: net::SocketAddr) -> io::Result<net::UdpSocket> {
    match peer {
        net::SocketAddr::V4(_) => net::UdpSocket::bind(("0.0.0.0", 0)),
        net::SocketAddr::V6(_) => net::UdpSocket::bind(("::", 0)),
    }
}
