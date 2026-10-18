# minipbx design

Status: agreed plan, not yet implemented beyond `siphora::grammar`.

## Layers

```
 ┌──────────── transport ─────────────┐       ┌─ transaction worker ─┐       ┌── TU ──┐
 UDP socket task ─┐                     │       │                      │       │        │
 TCP listener ──► connection task (each)┼──────►│ transactions (map)   │──────►│ B2BUA, │
                  framing + index      │ mpsc  │ + timer queue        │ mpsc  │ regis- │
                  ◄── TransportHandle ◄─┼───────│                      │◄──────│ trar   │
 └─────────────────────────────────────┘       └──────────────────────┘       └────────┘
```

Every arrow is a tokio `mpsc` channel or a cheap cloneable handle.

## Transport

- **UDP**: one task per socket runs the receive loop. Sending uses the
  same socket through `Arc<UdpSocket>`, so it needs no task of its own.
- **TCP (later TLS)**: one listener task, plus one task per connection
  that owns the read side and stream framing (by `Content-Length`).
  Writes reach a connection through its `mpsc` sender.
- **Connection table**: (remote address, transport) → connection handle,
  so responses go back over the connection the request arrived on
  (RFC 3261 §18.2.2).
- **Transport-owned work**:
  - framing: the datagram for UDP, `Content-Length` for streams
  - building the lazy message (below) and rejecting malformed ones with 400
  - adding `received` / `rport` to the top Via of incoming requests (§18.2.1)
  - routing responses by Via
  - CRLF keepalive
- **Output**: `Inbound { msg, source, transport, conn }` to the transaction
  worker over one `mpsc` channel.

## Lazy message

A message keeps the bytes it arrived in (`Bytes`). Parsing happens in
three levels:

1. **Index**: every header line is recorded as (name, byte range of the
   value). This costs nothing extra, since the header section has to be
   scanned anyway to find the body.
2. **Strict core**: the headers every layer needs are parsed right away:
   top Via, From, To, Call-ID, CSeq, Max-Forwards and Content-Length
   (the §8.1.1 set). If one is missing or malformed, the transport
   rejects the message. Messages that get past the transport therefore
   have infallible core accessors (`msg.call_id() -> &str`).
3. **Query**: any other header is parsed on demand through a
   `TypedHeader` trait (`msg.header::<Allow>()`), with the
   `siphora::grammar` combinators doing the parsing. A malformed
   non-core header is reported only when someone asks for it.

The original bytes are what the S3 archiver stores, verbatim.

Outgoing messages are built with a separate builder, which the B2BUA
uses for most of its traffic.

## Transaction worker

- **One worker task to start.** Later there will be N workers, with
  messages assigned by hashing Call-ID. A dialog's INVITE, CANCEL, ACK
  and responses all share a Call-ID, so they land on the same worker,
  and CANCEL-to-INVITE matching stays a local lookup.
- **Worker state**: `HashMap<TransactionKey, Transaction>` plus a timer
  queue. The key is the RFC 3261 §17.1.3 / §17.2.3 key: branch +
  sent-by + method, with ACK matched to its INVITE.
- **Worker loop**: a `select!` over three things: messages from the
  transport, commands from the TU, and the next timer deadline.
- **Strays** (2xx retransmissions, ACK for 2xx, responses that match no
  transaction) go straight to the TU.

### Sans-IO transactions

The four state machines (INVITE client, non-INVITE client, INVITE
server, non-INVITE server) are plain synchronous structs:

```rust
fn handle(&mut self, event: Event, out: &mut Vec<Action>);

enum Event<'a> { Received(&'a Message), Send(Message), Timer(TimerKind), TransportError }
enum Action { Transmit(Bytes), StartTimer(TimerKind, Duration), CancelTimer(TimerKind),
              Deliver(TuEvent), Terminated }
```

They never await, never touch a channel, and never read the clock. They
only return what they want done, and the worker carries it out. All of
RFC 3261 §17 lives in `handle`; the worker is plumbing. This means
timers A–K can be tested as plain function calls, with no runtime and
no waiting.

## Build order

1. Lazy message (index, strict core, `TypedHeader` queries) and the
   outgoing builder.
2. Transport: the UDP task, then TCP connection tasks, behind one
   `TransportHandle`.
3. Transaction state machines with tests, then the single worker.
4. TU interface, then a registrar as its first user.
