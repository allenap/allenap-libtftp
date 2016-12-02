# allenap's TFTP library for Rust

This library will let you build a read-only TFTP server
in [Rust](https://www.rust-lang.org/) with the following features:

 * [RFC-1350](https://tools.ietf.org/html/rfc1350) - The TFTP Protocol
   (Revision 2)

 * [RFC-2347](https://tools.ietf.org/html/rfc2347) - TFTP Option
   Extension

 * [RFC-2348](https://tools.ietf.org/html/rfc2348) - TFTP Blocksize
   Option

 * [RFC-2349](https://tools.ietf.org/html/rfc2349) - TFTP Timeout
   Interval and Transfer Size Options

 * `blkno` rollover, allowing tranfers of unlimited size.

The places to start are the top-level `serve` function, the `Handler`
trait, and the `rrq.serve` function.

The intent is to support writable servers, and clients. The code is
alpha level right now, and given time I would change quite a lot, but
for now this works.


## To do

 * Change all the `println!`s to
   use [slog](https://github.com/slog-rs/slog) or an equivalent logging
   library.

 * Fix the layering violation used to efficiently construct outgoing
   `DATA` packets.

 * Wait for `ACK` after sending `OACK`.

 * Support the `windowsize` option;
   see [RFC-7440](https://tools.ietf.org/html/rfc7440).

 * More unit tests.

 * Some integration tests.

 * Clean-ups, refactorings, and so on: it's kind of rough-n-ready right now.
