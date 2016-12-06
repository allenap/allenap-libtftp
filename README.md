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

## Thanks

I learned a lot from reading the code in Arjan
Topolovec's [rust-tftp](https://github.com/arjantop/rust-tftp). In an
ideal world I would have instead contributed back to that, and I may yet
do that, but I started on this project as a way to learn Rust.

I've read a lot of material about Rust and a lot of Rust code, but
*writing* code has been the best way to internalise that knowledge, and
to find out what I thought I knew but didn't. Starting from scratch with
*rust-tftp* as a guide has worked well for me.

Thank you Arjan Topolovec.


## To do

 * Fix the layering violation used to efficiently construct outgoing
   `DATA` packets.

 * Wait for `ACK` after sending `OACK`.

 * Support the `windowsize` option;
   see [RFC-7440](https://tools.ietf.org/html/rfc7440).

 * More unit tests.

 * Some integration tests.

 * Clean-ups, refactorings, and so on: it's kind of rough-n-ready right now.
