# cftp

cftp is a runtime-agnostic, asynchronous crate for building FTP servers in Rust. it provides a trait, `FtpHandler`, which allows you to entirely customise all aspects about your server. reads and writes are based on the `futures` crate's `AsyncRead` and `AsyncWrite` traits, allowing for file reads and writes to be performed in a streaming fashion.

## purpose

many modern-day services with relation to files (i.e. game server hosting, file storage, and even VPSes occasionally) are expected to have first-class FTP support. the purpose of cftp is to allow people to build these services without having to deal with the archaic, highly non-standard FTP protocol. if you can list, read, write and delete files with a given service, you can integrate it with cftp.

cftp is also entirely generic over the stream it uses, as long as it can be read from and written into asynchronously. this means you're not purely restricted to using TCP for your FTP server, and it also makes the server very easy to test.

## protocol support

cftp is not production-ready yet. many commands which are a part of FTP currently remain unsupported, although most base features expected of an FTP server will work fine on the protocol-level.

## examples

you can find an example on how to set up a tcp-based ftp server under the `crates/runner` directory. there's some comments dotted about it, which i highly recommend you read.
