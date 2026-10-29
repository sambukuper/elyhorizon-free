//! The quiclime control channel: messages are JSON with a `kind` field,
//! framed by a varint length, one message per frame, both ways. The client
//! asks for a domain, the relay answers with one and then opens one
//! bidirectional stream per guest, which this crate bridges to the world.

/// The serverbound messages this client sends (e4mc's
/// `ControlMessageCodec`, the subset a host needs).
#[derive(Debug, PartialEq, Eq)]
pub enum Serverbound {
    ProbeCapabilities,
    RequestDomainAssignment,
}

/// The clientbound messages (e4mc's clientbound control messages).
#[derive(Debug, PartialEq, Eq)]
pub enum Clientbound {
    /// The relay's capabilities (e4mc probes them before registering).
    HasCapabilities(Vec<String>),
    /// The world's public address is ready.
    DomainAssignmentComplete(String),
    /// A relay-wide note the player should see.
    RequestMessageBroadcast(String),
    TicketRegistered,
    UnknownMessage,
}

impl Serverbound {
    pub fn to_json(&self) -> Vec<u8> {
        let kind = match self {
            Self::ProbeCapabilities => "probe_capabilities",
            Self::RequestDomainAssignment => "request_domain_assignment",
        };
        format!("{{\"kind\":\"{kind}\"}}").into_bytes()
    }
}

/// One frame: varint length + the JSON body.
pub fn frame(message: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(message.len() + 5);
    let mut length = message.len();
    loop {
        let byte = (length & 0x7f) as u8;
        length >>= 7;
        if length == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
    out.extend_from_slice(message);
    out
}

/// A streaming decoder for relay frames: feed it whatever arrived, it hands
/// back complete messages. `Err` on a frame that claims an impossible size.
#[derive(Default)]
pub struct Decoder {
    buf: Vec<u8>,
}

impl Decoder {
    pub fn feed(&mut self, bytes: &[u8]) -> Result<Vec<Clientbound>, &'static str> {
        self.buf.extend_from_slice(bytes);
        let mut out = Vec::new();
        loop {
            let Some((length, header)) = varint(&self.buf) else {
                return Ok(out);
            };
            if length > 8192 {
                return Err("the relay sent an oversized control frame");
            }
            let end = header + length;
            if self.buf.len() < end {
                return Ok(out);
            }
            let body = &self.buf[header..end];
            let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) else {
                self.buf.drain(..end);
                out.push(Clientbound::UnknownMessage);
                continue;
            };
            let message = match json.get("kind").and_then(|k| k.as_str()) {
                Some("has_capabilities") => Clientbound::HasCapabilities(
                    json.get("caps")
                        .and_then(|c| c.as_array())
                        .map(|caps| {
                            caps.iter()
                                .filter_map(|c| c.as_str().map(str::to_owned))
                                .collect()
                        })
                        .unwrap_or_default(),
                ),
                Some("domain_assignment_complete") => {
                    match json.get("domain").and_then(|d| d.as_str()) {
                        Some(domain) => Clientbound::DomainAssignmentComplete(domain.to_owned()),
                        None => return Err("the relay's domain frame holds no domain"),
                    }
                }
                Some("request_message_broadcast") => Clientbound::RequestMessageBroadcast(
                    json.get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or_default()
                        .to_owned(),
                ),
                Some("ticket_registered") => Clientbound::TicketRegistered,
                _ => Clientbound::UnknownMessage,
            };
            self.buf.drain(..end);
            out.push(message);
        }
    }
}

/// A protocol varint: the value and how many bytes it took.
fn varint(buf: &[u8]) -> Option<(usize, usize)> {
    let mut value = 0usize;
    for (i, &b) in buf.iter().take(5).enumerate() {
        value |= usize::from(b & 0x7f) << (i * 7);
        if b & 0x80 == 0 {
            return Some((value, i + 1));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_round_trip_through_the_decoder() {
        let mut decoder = Decoder::default();
        let messages = decoder
            .feed(
                &[
                    frame(br#"{"kind":"has_capabilities","caps":["dialtone_sidecar"]}"#),
                    frame(br#"{"kind":"domain_assignment_complete","domain":"gone.e4mc.link"}"#),
                    frame(br#"{"kind":"request_message_broadcast","message":"hello"}"#),
                    frame(br#"{"kind":"ticket_registered"}"#),
                ]
                .concat(),
            )
            .unwrap();
        assert_eq!(
            messages,
            vec![
                Clientbound::HasCapabilities(vec!["dialtone_sidecar".into()]),
                Clientbound::DomainAssignmentComplete("gone.e4mc.link".into()),
                Clientbound::RequestMessageBroadcast("hello".into()),
                Clientbound::TicketRegistered,
            ]
        );
    }

    #[test]
    fn frames_arriving_in_pieces_decode() {
        let whole = frame(br#"{"kind":"domain_assignment_complete","domain":"gone.e4mc.link"}"#);
        let mut decoder = Decoder::default();
        let mut messages = Vec::new();
        for chunk in whole.chunks(3) {
            messages.extend(decoder.feed(chunk).unwrap());
        }
        assert_eq!(
            messages,
            vec![Clientbound::DomainAssignmentComplete(
                "gone.e4mc.link".into()
            )]
        );
    }

    #[test]
    fn unknown_kinds_and_broken_json_become_unknown() {
        let mut decoder = Decoder::default();
        let messages = decoder
            .feed(&[frame(br#"{"kind":"frobnicate"}"#), frame(b"not json")].concat())
            .unwrap();
        assert_eq!(messages.len(), 2);
        assert!(matches!(messages[0], Clientbound::UnknownMessage));
        assert!(matches!(messages[1], Clientbound::UnknownMessage));
    }

    #[test]
    fn oversized_frames_are_refused() {
        let mut decoder = Decoder::default();
        assert!(decoder.feed(&[0xff, 0xff, 0xff, 0xff, 0x7f]).is_err());
    }

    #[test]
    fn serverbound_messages_serialize_like_the_java_mods() {
        assert_eq!(
            Serverbound::ProbeCapabilities.to_json(),
            br#"{"kind":"probe_capabilities"}"#.to_vec()
        );
        assert_eq!(
            Serverbound::RequestDomainAssignment.to_json(),
            br#"{"kind":"request_domain_assignment"}"#.to_vec()
        );
    }

    #[test]
    fn frames_carry_multi_byte_lengths() {
        // A domain long enough that its frame's varint length takes two bytes.
        let domain = format!("{}{}", "x".repeat(266), ".e4mc.link");
        let body = format!(r#"{{"kind":"domain_assignment_complete","domain":"{domain}"}}"#);
        let framed = frame(body.as_bytes());
        assert!(framed.len() > 255);
        let mut decoder = Decoder::default();
        let messages = decoder.feed(&framed).unwrap();
        assert_eq!(
            messages,
            vec![Clientbound::DomainAssignmentComplete(domain)]
        );
        assert!(decoder.buf.is_empty(), "the whole frame should be consumed");
    }
}
