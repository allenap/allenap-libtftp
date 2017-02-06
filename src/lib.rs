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


/// Starts a TFTP server at the given address.
///
/// Well-formed requests are passed to `handler`, and all logging is
/// handled by `logger`.
pub fn serve(
    addr: net::SocketAddr, handler: &Handler, logger: &slog::Logger)
    -> io::Result<()>
{
    let socket = net::UdpSocket::bind(addr)?;
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


/// A TFTP handler to which requests are passed once they've been
/// parsed. A handler can choose to ignore, reject (with an error), or
/// serve each request that comes in.
pub trait Handler {

    /// Handle a new, well-formed, TFTP request.
    ///
    /// The default implementation calls
    /// [`handle_rrq`](#method.handle_rrq) for a read request and
    /// [`handle_wrq`](#method.handle_wrq) for a write request, and
    /// [`handle_other`](#method.handle_other) for everything else.
    ///
    /// In case of an error, this can return a `Packet` representing the
    /// error to be sent to the other side. For example:
    ///
    /// ```
    /// # use allenap_libtftp::packet;
    /// Some(packet::Packet::Error(
    ///     packet::ErrorCode::AccessViolation,
    ///     packet::ErrorMessage("read not supported".to_owned()),
    /// ));
    /// ```
    ///
    /// Use this when the error occurs prior the commencing the
    /// transfer; once the transfer has begin, send errors via the
    /// channel created for the transfer.
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

    /// Handle a read request (`RRQ`).
    ///
    /// By default this is rejected as an access violation. Implementors
    /// can define something more interesting.
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

    /// Handle a write request (`WRQ`).
    ///
    /// By default this is rejected as an access violation. Implementors
    /// can define something more interesting.
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

    /// Handle all other requests.
    ///
    /// By default these are completely ignored. The TFTP specs do not
    /// define request types other than `RRQ` and `WRQ` so this might be
    /// a misdirected or corrupted packet. Implementors may want to log
    /// this.
    fn handle_other(
        &self, _local: net::SocketAddr, _remote: net::SocketAddr,
        _packet: Packet)
        -> Option<Packet>
    {
        None  // Ignore.
    }

}


/// Bind a new UDP socket at the given address.
fn make_socket(peer: net::SocketAddr) -> io::Result<net::UdpSocket> {
    match peer {
        net::SocketAddr::V4(_) => net::UdpSocket::bind(("0.0.0.0", 0)),
        net::SocketAddr::V6(_) => net::UdpSocket::bind(("::", 0)),
    }
}
