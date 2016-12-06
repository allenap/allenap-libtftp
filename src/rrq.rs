extern crate byteorder;
extern crate slog;

use std::fs;
use std::net;
use std::io;
use std::time;

use super::packet::{
    BlockNum,
    Data,
    ErrorCode,
    ErrorMessage,
    Filename,
    Packet,
    TransferMode,
};
use super::options::Options;
use super::make_socket;


pub fn serve_file(
    peer: net::SocketAddr,
    filename: Filename,
    txmode: TransferMode,
    options: Options,
    logger: &slog::Logger,
) {
    info!(logger, "Received RRQ: {:?} {:?} {:?}", filename, txmode, options);
    let Filename(filename) = filename;
    match make_socket(peer) {
        Ok(socket) => match fs::File::open(&filename) {
            Ok(mut file) => {
                let len = file.metadata().ok().and_then(|m| Some(m.len()));
                let logger = logger.new(o!(
                    "peer" => format!("{}", peer),
                    "filename" => filename,
                ));
                match send_to(
                    &mut file, len, socket, peer, options, &logger) {
                    Ok(_) => info!(
                        logger, "Completed transfer to {:?}", peer),
                    Err(error) => error!(
                        logger, "Error transferring to {:?}: {}", peer, error),
                };
            },
            Err(error) => {
                error!(logger, "Problem with file {}: {}", &filename, error);
                // TODO: Send error to peer.
            },
        },
        Err(error) => {
            error!(logger, "Could not open socket: {}", error);
        },
    };
}


const EMPTY_DATA: Data<'static> = Data(&[]);


fn send_to(
    data: &mut io::Read,
    len: Option<u64>,
    socket: net::UdpSocket,
    peer: net::SocketAddr,
    options: Options,
    logger: &slog::Logger,
)
    -> io::Result<()>
{
    // First, connect the socket to the peer so that we're only sending
    // and receiving traffic to/from the peer. TODO: Do this earlier?
    socket.connect(peer)?;

    let mut options_out = Options::new();

    let blksize: usize = match options.blksize {
        Some(blksize) if blksize >= 512 => {
            options_out.blksize = Some(blksize);
            blksize as usize
        },
        _ => 512,  // Default.
    };

    socket.set_read_timeout(
        Some(match options.timeout {
            Some(timeout) if timeout >= 1 => {
                options_out.timeout = Some(timeout);
                time::Duration::from_secs(timeout as u64)
            },
            _ => {
                time::Duration::from_secs(8u64)  // Default.
            },
        })
    )?;

    match options.tsize {
        Some(0) => {
            options_out.tsize = len;
        },
        Some(tsize) => {
            warn!(logger, "Option tsize should be zero, got: {}", tsize);
        },
        None => {
            // Do nothing.
        },
    };

    let mut bufout = vec![0u8; 4 + blksize];  // opcode + blkno + data
    let mut bufin = vec![0u8; blksize];

    if options_out.is_set() {
        let packet = Packet::OAck(options_out);
        let size = packet.write(&mut bufout)?;
        socket.send(&bufout[..size])?;
        info!(logger, "Sent OACK ({} bytes) to {}.", size, &peer);
        // TODO: Wait for ACK(0).
    }

    fn timed_out(error: &io::Error) -> bool {
        // See the comment in UdpSocket.set_{read,write}_timeout to
        // understand why both errors are matched.
        error.kind() == io::ErrorKind::WouldBlock ||
            error.kind() == io::ErrorKind::TimedOut
    }

    'send: for blkno in (1 as u16).. {
        let mut timeouts = 0u8;
        match data.read(&mut bufout[4..]) {
            Ok(size) => {
                // To avoid an extra copy we cheat and use a Data packet
                // to write headers only. We've already read the payload
                // into the correct place in `bufout`.
                let packet = Packet::Data(BlockNum(blkno), EMPTY_DATA);
                packet.write(&mut bufout[..4])?;
                socket.send(&bufout[..size + 4])?;
                info!(logger, "Sent DATA ({} bytes) to {}.", size, &peer);

                'recv: loop {
                    match socket.recv(&mut bufin) {
                        Ok(amt) => {
                            match Packet::parse(&mut bufin[..amt]) {
                                Ok(packet) => match packet {
                                    Packet::Ack(BlockNum(blocknum)) => {
                                        if blocknum == blkno {
                                            break 'recv;
                                        };
                                    },
                                    Packet::Error(code, message) => {
                                        error!(logger, "{:?}: {:?}", code, message);
                                        break 'send;
                                    },
                                    Packet::Data(..) => warn!(
                                        logger, "Ignoring unexpected DATA packet."),
                                    Packet::Read(..) => warn!(
                                        logger, "Ignoring unexpected RRQ packet."),
                                    Packet::Write(..) => warn!(
                                        logger, "Ignoring unexpected WRQ packet."),
                                    Packet::OAck(..) => warn!(
                                        logger, "Ignoring unexpected OACK packet."),
                                },
                                Err(error) => {
                                    warn!(
                                        logger, "Ignoring mangled packet ({:?}).",
                                        error);
                                },
                            };
                        },
                        Err(ref error) if timed_out(error) => {
                            match timeouts {
                                0...7 => {
                                    timeouts += 1;
                                    socket.send(&bufout[..size + 4])?;
                                    info!(
                                        logger,
                                        "Sent DATA ({} bytes) to {} (attempt #{}).",
                                        size, &peer, timeouts + 1);
                                },
                                _ => {
                                    error!(logger, "Too many time-outs; aborting");
                                    break 'send;
                                },
                            };
                        },
                        Err(error) => {
                            error!(logger, "Error receiving packet: {}", error);
                            break 'send;
                        },
                    }
                }

                if size < blksize {
                    break;
                }
            },
            Err(error) => {
                let packet = Packet::Error(
                    ErrorCode::NotDefined, ErrorMessage(format!(
                        "Something broke: {}\0", error)));

                match packet.write(&mut bufout) {
                    Ok(length) => {
                        socket.send(&bufout[..length])?;
                    },
                    Err(error) => {
                        error!(
                            logger, "Error preparing error packet: {:?}",
                            error);
                    },
                };

                break 'send;
            },
        }
    };
    Result::Ok(())
}
