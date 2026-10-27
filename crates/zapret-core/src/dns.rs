//! Minimal DNS response builder for the Android VPN DNS server.
//! A hosts-map override: answer A queries from a JSON map, else defer upstream.

use std::collections::HashMap;

const TYPE_A: u16 = 1;
const TYPE_AAAA: u16 = 28;

/// Parse our hosts block (`# ZAPRET-GUI ...` file body: lines `ip host`)
/// into `{"host": ["ip", ...]}` JSON.
pub fn hosts_block_to_json(block: &str) -> String {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for line in block.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let (Some(ip), Some(host)) = (parts.next(), parts.next()) else {
            continue;
        };
        if ip.parse::<std::net::Ipv4Addr>().is_err() {
            continue;
        }
        let e = map.entry(host.to_lowercase()).or_default();
        if !e.iter().any(|x| x == ip) {
            e.push(ip.to_string());
        }
    }
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".into())
}

fn parse_hosts_json(json: &str) -> HashMap<String, Vec<String>> {
    serde_json::from_str(json).unwrap_or_default()
}

/// Build a DNS response for `query` if the qname is in the hosts map.
/// Returns None => caller should forward upstream.
///
/// - A (1) on a known host: N answers (TTL 60)
/// - any other type on a known host: NOERROR/0 answers (prevents poisoned
///   AAAA/SVCB from upstream for overridden hosts)
/// - unknown host: None (forward)
/// - malformed / QDCOUNT != 1: None (forward)
pub fn dns_respond(query: &[u8], hosts_json: &str) -> Option<Vec<u8>> {
    if query.len() < 12 {
        return None;
    }
    let qdcount = u16::from_be_bytes([query[4], query[5]]);
    if qdcount != 1 {
        return None;
    }

    // parse QNAME (allow compression pointers inside the read name)
    let (name, name_end) = read_name(query, 12)?;
    let pos = name_end;
    if pos + 4 > query.len() {
        return None;
    }
    let qtype = u16::from_be_bytes([query[pos], query[pos + 1]]);
    let qclass = u16::from_be_bytes([query[pos + 2], query[pos + 3]]);
    if qclass != 1 {
        return None;
    }
    let question_end = pos + 4;

    let hosts = parse_hosts_json(hosts_json);
    let key = name.to_lowercase();
    let ips: &[String] = match hosts.get(&key) {
        Some(v) => v,
        None => return None,
    };

    let mut out = Vec::with_capacity(64 + ips.len() * 16);
    // header: id echo, flags: QR | copy RD | RA, qd=1, an=count
    let id = [query[0], query[1]];
    let rd = query[2] & 0x01;
    let flags: u16 = 0x8080 | ((rd as u16) << 8); // QR=1, RD=copy, RA=1
    out.extend_from_slice(&id);
    out.extend_from_slice(&flags.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes()); // QDCOUNT
    let ancount: u16 = if qtype == TYPE_A {
        ips.len() as u16
    } else {
        0
    };
    out.extend_from_slice(&ancount.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes()); // NSCOUNT
    out.extend_from_slice(&0u16.to_be_bytes()); // ARCOUNT
                                                // echo question section verbatim
    out.extend_from_slice(&query[12..question_end]);

    if qtype == TYPE_A {
        for ip in ips {
            let octets: std::net::Ipv4Addr = ip.parse().ok()?;
            // name pointer to offset 12 (compression)
            out.extend_from_slice(&[0xC0, 0x0C]);
            out.extend_from_slice(&TYPE_A.to_be_bytes());
            out.extend_from_slice(&1u16.to_be_bytes()); // IN
            out.extend_from_slice(&60u32.to_be_bytes()); // TTL
            out.extend_from_slice(&4u16.to_be_bytes());
            out.extend_from_slice(&octets.octets());
        }
    }
    // AAAA/other on known host: empty answer already encoded (ancount=0)
    let _ = TYPE_AAAA;
    Some(out)
}

/// Read a DNS name starting at `start`. Returns (name, offset_after_name_in_this_message).
/// Follows compression pointers for reading; output name is dot-separated labels.
fn read_name(msg: &[u8], start: usize) -> Option<(String, usize)> {
    let mut labels = Vec::new();
    let mut pos = start;
    let mut end: Option<usize> = None; // position after name in the original stream (first traverse)
    let mut hops = 0;
    loop {
        if pos >= msg.len() {
            return None;
        }
        let len = msg[pos];
        if len == 0 {
            pos += 1;
            if end.is_none() {
                end = Some(pos);
            }
            break;
        }
        if len & 0xC0 == 0xC0 {
            if pos + 1 >= msg.len() {
                return None;
            }
            let ptr = (((len as usize) & 0x3F) << 8) | msg[pos + 1] as usize;
            if end.is_none() {
                end = Some(pos + 2);
            }
            pos = ptr;
            hops += 1;
            if hops > 16 {
                return None;
            }
            continue;
        }
        if len & 0xC0 != 0 {
            return None; // reserved
        }
        let l = len as usize;
        if pos + 1 + l > msg.len() {
            return None;
        }
        labels.push(String::from_utf8_lossy(&msg[pos + 1..pos + 1 + l]).into_owned());
        pos += 1 + l;
        if hops > 0 && end.is_some() {
            // we're inside a jumped segment; keep consuming there
        }
    }
    Some((labels.join("."), end?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_query(name: &str, qtype: u16) -> Vec<u8> {
        let mut q = vec![0x12, 0x34, 0x01, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
        for label in name.split('.') {
            q.push(label.len() as u8);
            q.extend_from_slice(label.as_bytes());
        }
        q.push(0);
        q.extend_from_slice(&qtype.to_be_bytes());
        q.extend_from_slice(&1u16.to_be_bytes());
        q
    }

    fn hosts() -> String {
        hosts_block_to_json(
            "# ZAPRET-GUI ROBLOX BEGIN\n# updated: x\n1.2.3.4 tr.rbxcdn.com\n5.6.7.8 tr.rbxcdn.com\n# ZAPRET-GUI ROBLOX END",
        )
    }

    #[test]
    fn block_to_json_skips_comments_and_groups() {
        let j = hosts();
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["tr.rbxcdn.com"], serde_json::json!(["1.2.3.4", "5.6.7.8"]));
    }

    #[test]
    fn a_query_known_host_answers() {
        let q = build_query("tr.rbxcdn.com", TYPE_A);
        let r = dns_respond(&q, &hosts()).expect("should answer");
        assert_eq!(&r[0..2], &q[0..2], "id echo");
        assert_eq!(r[2] & 0x80, 0x80, "QR set");
        let ancount = u16::from_be_bytes([r[6], r[7]]);
        assert_eq!(ancount, 2);
        // each answer: ptr(2) type(2) class(2) ttl(4) rdlen(2) ip(4) = 16 bytes
        let n = r.len();
        assert_eq!(&r[n - 22..n - 20], &[0, 4], "rdlength #1");
        assert_eq!(&r[n - 20..n - 16], &[1, 2, 3, 4], "ip #1");
        assert_eq!(&r[n - 6..n - 4], &[0, 4], "rdlength #2");
        assert_eq!(&r[n - 4..], &[5, 6, 7, 8], "ip #2");
    }

    #[test]
    fn unknown_host_defers() {
        let q = build_query("discord.com", TYPE_A);
        assert!(dns_respond(&q, &hosts()).is_none());
    }

    #[test]
    fn aaaa_on_known_host_empty_nodata() {
        let q = build_query("TR.RBXCDN.COM", TYPE_AAAA); // case-insensitive
        let r = dns_respond(&q, &hosts()).expect("NODATA answer");
        let ancount = u16::from_be_bytes([r[6], r[7]]);
        assert_eq!(ancount, 0);
        assert_eq!(r[2] & 0x80, 0x80);
        assert_eq!(r[3] & 0x0F, 0, "NOERROR");
    }

    #[test]
    fn malformed_defers() {
        assert!(dns_respond(&[0u8; 5], &hosts()).is_none());
        let mut q = build_query("a.b", TYPE_A);
        q[4] = 0; // QDCOUNT=0
        assert!(dns_respond(&q, &hosts()).is_none());
    }
}
