# minipbx

An experimental PBX in Rust, built from scratch: SIP signaling, RTP media, and
S3-only storage for SIP messages and call recordings. The goal is to find out
whether that storage design holds up.

## Layout

| Crate | Role |
|---|---|
| [`crates/siphora`](crates/siphora) | SIP: message parser, then transport, transactions, dialogs |
| [`crates/octothorpe`](crates/octothorpe) | RTP: sockets, packet parsing, relay |
| [`crates/minipbx`](crates/minipbx) | The PBX binary: registrar, B2BUA, S3 archiving and recording |

`siphora` and `octothorpe` know nothing about S3 or the PBX, so they stay
usable as standalone libraries.

## Roadmap

1. **SIP message** (done): parse and serialize requests and responses, typed
   Via / CSeq / From / To / Contact.
2. **SIP transport**: UDP socket on tokio.
3. **SIP transactions** (RFC 3261 §17): client and server state machines with
   timers, written as pure event → action functions so they test without a network.
4. **Registrar and B2BUA** in `minipbx`, with in-memory registrations.
5. **SDP**: offer/answer for PCMU and PCMA only.
6. **RTP relay** in `octothorpe`.
7. **S3 storage**: per-call SIP archive objects and multipart-uploaded
   recordings, written off the signaling path through bounded channels.

Out of scope for now: TCP/TLS, SRTP, NAT traversal, transcoding.

## Development

```sh
cargo test
```

Useful tools: SIPp, baresip or Linphone, sngrep, and MinIO as a local S3.
