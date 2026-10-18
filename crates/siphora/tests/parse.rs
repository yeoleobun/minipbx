use siphora::{Message, Method, NameAddr, ParseError, Request, Response, StatusCode, Uri, Via};

fn crlf(s: &str) -> Vec<u8> {
    s.replace('\n', "\r\n").into_bytes()
}

fn request(raw: &str) -> Request {
    match Message::parse(&crlf(raw)).unwrap() {
        Message::Request(r) => r,
        m => panic!("expected request, got {m:?}"),
    }
}

fn response(raw: &str) -> Response {
    match Message::parse(&crlf(raw)).unwrap() {
        Message::Response(r) => r,
        m => panic!("expected response, got {m:?}"),
    }
}

// RFC 3665 §3.2, F1.
const INVITE: &str = "\
INVITE sip:bob@biloxi.example.com SIP/2.0
Via: SIP/2.0/TCP client.atlanta.example.com:5060;branch=z9hG4bK74bf9
Max-Forwards: 70
From: Alice <sip:alice@atlanta.example.com>;tag=9fxced76sl
To: Bob <sip:bob@biloxi.example.com>
Call-ID: 3848276298220188511@atlanta.example.com
CSeq: 1 INVITE
Contact: <sip:alice@client.atlanta.example.com;transport=tcp>
Content-Type: application/sdp
Content-Length: 10

v=0
o=x y
";

#[test]
fn parses_invite() {
    let req = request(INVITE);
    assert_eq!(req.method, Method::Invite);
    assert_eq!(req.uri.user.as_deref(), Some("bob"));
    assert_eq!(req.uri.host, "biloxi.example.com");
    assert_eq!(req.body, b"v=0\r\no=x y");

    let h = &req.headers;
    assert_eq!(
        h.call_id().unwrap(),
        "3848276298220188511@atlanta.example.com"
    );
    let cseq = h.cseq().unwrap();
    assert_eq!((cseq.seq, cseq.method), (1, Method::Invite));

    let via = h.top_via().unwrap();
    assert_eq!(via.transport, "TCP");
    assert_eq!(via.host, "client.atlanta.example.com");
    assert_eq!(via.port, Some(5060));
    assert_eq!(via.branch(), Some("z9hG4bK74bf9"));

    let from = h.from().unwrap();
    assert_eq!(from.display_name.as_deref(), Some("Alice"));
    assert_eq!(from.tag(), Some("9fxced76sl"));
    assert_eq!(h.to().unwrap().tag(), None);

    let contact = &h.contacts().unwrap()[0];
    assert_eq!(contact.uri.params.get("transport"), Some("tcp"));
}

#[test]
fn round_trips() {
    let msg = Message::parse(&crlf(INVITE)).unwrap();
    let again = Message::parse(&msg.to_bytes()).unwrap();
    assert_eq!(msg, again);
}

#[test]
fn parses_response_with_multiple_vias() {
    let res = response(
        "\
SIP/2.0 180 Ringing
Via: SIP/2.0/UDP proxy.example.com;branch=z9hG4bK1, SIP/2.0/UDP 10.0.0.1:5062;branch=z9hG4bK2;received=192.0.2.4
Via: SIP/2.0/UDP pc33.example.com;branch=z9hG4bK3
To: <sip:bob@example.com>;tag=8321234356
From: <sip:alice@example.com>;tag=1928301774
Call-ID: a84b4c76e66710
CSeq: 314159 INVITE
Content-Length: 0

",
    );
    assert_eq!(res.status, StatusCode::RINGING);
    assert_eq!(res.reason, "Ringing");
    let vias = res.headers.vias().unwrap();
    let branches: Vec<_> = vias.iter().map(|v| v.branch().unwrap()).collect();
    assert_eq!(branches, ["z9hG4bK1", "z9hG4bK2", "z9hG4bK3"]);
    assert_eq!(vias[1].params.get("received"), Some("192.0.2.4"));
    assert!(res.body.is_empty());
}

#[test]
fn expands_compact_headers_and_unfolds_lines() {
    let req = request(
        "\
OPTIONS sip:example.com SIP/2.0
v: SIP/2.0/UDP host.example.com;branch=z9hG4bKx
i: abc123
f: <sip:a@example.com>;tag=1
t: <sip:b@example.com>
l: 0
Subject: I know you're there,
   pick up the phone
cseq: 2 OPTIONS

",
    );
    let h = &req.headers;
    assert_eq!(h.call_id().unwrap(), "abc123");
    assert_eq!(h.get("Call-ID"), h.get("i"));
    assert_eq!(h.cseq().unwrap().seq, 2);
    assert_eq!(
        h.get("subject"),
        Some("I know you're there, pick up the phone")
    );
    let names: Vec<_> = h.iter().map(|(k, _)| k).collect();
    assert_eq!(
        names,
        [
            "Via",
            "Call-ID",
            "From",
            "To",
            "Content-Length",
            "Subject",
            "CSeq"
        ]
    );
}

#[test]
fn splits_contacts_respecting_quotes() {
    let req = request(
        "\
REGISTER sip:registrar.example.com SIP/2.0
Contact: \"Doe, John\" <sip:john@10.0.0.1>;expires=60, sip:john@10.0.0.2;q=0.5
Content-Length: 0

",
    );
    let contacts = req.headers.contacts().unwrap();
    assert_eq!(contacts.len(), 2);
    assert_eq!(contacts[0].display_name.as_deref(), Some("Doe, John"));
    assert_eq!(contacts[0].params.get("expires"), Some("60"));
    // Without <>, ;q= belongs to the header, not the URI.
    assert_eq!(contacts[1].params.get("q"), Some("0.5"));
    assert!(contacts[1].uri.params.is_empty());
}

#[test]
fn body_without_content_length_takes_the_rest() {
    let req = request("MESSAGE sip:a@example.com SIP/2.0\nCall-ID: x\n\nhello");
    assert_eq!(req.body, b"hello");
}

#[test]
fn extra_datagram_bytes_are_discarded() {
    let req = request("MESSAGE sip:a@example.com SIP/2.0\nContent-Length: 2\n\nhello");
    assert_eq!(req.body, b"he");
}

#[test]
fn short_body_is_an_error() {
    let err = Message::parse(&crlf(
        "MESSAGE sip:a@example.com SIP/2.0\nContent-Length: 9\n\nhello",
    ));
    assert_eq!(
        err,
        Err(ParseError::BodyLength {
            expected: 9,
            actual: 5
        })
    );
}

#[test]
fn accepts_bare_lf_and_leading_empty_lines() {
    let raw = b"\r\n\r\nBYE sip:a@example.com SIP/2.0\nCall-ID: x\n\n";
    assert!(matches!(Message::parse(raw), Ok(Message::Request(_))));
}

#[test]
fn rejects_bad_start_lines() {
    for raw in [
        "INVITE sip:a@example.com SIP/3.0\n\n",
        "INVITE  sip:a@example.com SIP/2.0\n\n",
        "INVITE sip:a@example.com\n\n",
        "SIP/2.0 99 Too Low\n\n",
        "SIP/2.0 abc OK\n\n",
        "INV(TE sip:a@example.com SIP/2.0\n\n",
    ] {
        assert!(
            matches!(Message::parse(&crlf(raw)), Err(ParseError::StartLine(_))),
            "{raw:?}"
        );
    }
}

#[test]
fn incomplete_without_blank_line() {
    let err = Message::parse(b"INVITE sip:a@example.com SIP/2.0\r\nCall-ID: x\r\n");
    assert_eq!(err, Err(ParseError::Incomplete));
}

#[test]
fn parses_uris() {
    let uri: Uri = "sips:alice:secret@[2001:db8::1]:5061;transport=tls;lr?subject=hi"
        .parse()
        .unwrap();
    assert_eq!(uri.user.as_deref(), Some("alice"));
    assert_eq!(uri.password.as_deref(), Some("secret"));
    assert_eq!(uri.host, "[2001:db8::1]");
    assert_eq!(uri.port, Some(5061));
    assert_eq!(uri.params.get("transport"), Some("tls"));
    assert!(uri.params.contains("lr"));
    assert_eq!(uri.headers.as_deref(), Some("subject=hi"));
    assert_eq!(
        uri.to_string(),
        "sips:alice:secret@[2001:db8::1]:5061;transport=tls;lr?subject=hi"
    );

    assert!("tel:+15551234".parse::<Uri>().is_err());
    assert!("sip:@example.com".parse::<Uri>().is_err());
    assert!("sip:example.com:port".parse::<Uri>().is_err());
}

#[test]
fn via_allows_whitespace_in_protocol() {
    let via: Via = "SIP / 2.0 / udp 192.0.2.1:5060 ;branch=z9hG4bKabc"
        .parse()
        .unwrap();
    assert_eq!(via.transport, "UDP");
    assert_eq!(via.host, "192.0.2.1");
    assert_eq!(via.branch(), Some("z9hG4bKabc"));
}

#[test]
fn name_addr_display_escapes_quotes() {
    let mut addr = NameAddr::new("sip:a@example.com".parse().unwrap());
    addr.display_name = Some("say \"hi\"".into());
    addr.params.set("tag", Some("1"));
    let text = addr.to_string();
    assert_eq!(text, "\"say \\\"hi\\\"\" <sip:a@example.com>;tag=1");
    assert_eq!(text.parse::<NameAddr>().unwrap(), addr);
}

#[test]
fn builds_response_with_body() {
    let mut res = Response::new(StatusCode::OK);
    res.headers.push("Call-ID", "x");
    res.set_body(b"abc".to_vec());
    assert_eq!(
        res.to_bytes(),
        b"SIP/2.0 200 OK\r\nCall-ID: x\r\nContent-Length: 3\r\n\r\nabc"
    );
}
