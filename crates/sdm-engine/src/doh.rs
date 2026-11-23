//! Name lookups over DNS-over-HTTPS (RFC 8484): wire-format A and AAAA questions sent as
//! `application/dns-message` POSTs to the address the user picked.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reqwest::dns::{Addrs, Name, Resolve, Resolving};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const TYPE_A: u16 = 1;
const TYPE_AAAA: u16 = 28;
const MIN_TTL: u32 = 30;
const MAX_TTL: u32 = 300;

pub(crate) struct DohResolver(Arc<Inner>);

struct Inner {
    url: String,
    client: reqwest::Client,
    cache: Mutex<HashMap<String, (Instant, Vec<IpAddr>)>>,
}

impl DohResolver {
    pub(crate) fn new(url: String, client: reqwest::Client) -> Arc<DohResolver> {
        Arc::new(DohResolver(Arc::new(Inner { url, client, cache: Mutex::new(HashMap::new()) })))
    }
}

impl Inner {

    async fn ask(&self, host: &str, qtype: u16) -> Result<(Vec<IpAddr>, u32), BoxError> {
        let resp = self
            .client
            .post(&self.url)
            .header("content-type", "application/dns-message")
            .header("accept", "application/dns-message")
            .body(encode_query(host, qtype)?)
            .send()
            .await
            .map_err(|e| format!("the DNS server could not be reached: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("the DNS server answered {}", resp.status().as_u16()).into());
        }
        let body = resp.bytes().await.map_err(|e| format!("the DNS answer was cut off: {e}"))?;
        parse_answer(&body)
    }

    async fn lookup(self: Arc<Self>, host: String) -> Result<Vec<IpAddr>, BoxError> {
        if let Some((until, ips)) = self.cache.lock().unwrap().get(&host)
            && *until > Instant::now()
        {
            return Ok(ips.clone());
        }
        let (v4, v6) = tokio::join!(self.ask(&host, TYPE_A), self.ask(&host, TYPE_AAAA));
        let mut ips = Vec::new();
        let mut ttl = MAX_TTL;
        let mut error = None;
        for result in [v4, v6] {
            match result {
                Ok((found, t)) => {
                    if !found.is_empty() {
                        ttl = ttl.min(t);
                    }
                    ips.extend(found);
                }
                Err(e) => error = Some(e),
            }
        }
        if ips.is_empty() {
            return Err(error.unwrap_or_else(|| format!("no address found for {host}").into()));
        }
        let until = Instant::now() + Duration::from_secs(u64::from(ttl.clamp(MIN_TTL, MAX_TTL)));
        self.cache.lock().unwrap().insert(host, (until, ips.clone()));
        Ok(ips)
    }
}

impl Resolve for DohResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().trim_end_matches('.').to_ascii_lowercase();
        let me = self.0.clone();
        Box::pin(async move {
            // Local names never leave the machine.
            if host == "localhost" || host.ends_with(".local") || !host.contains('.') {
                let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
                return Ok(Box::new(addrs.into_iter()) as Addrs);
            }
            let ips = me.lookup(host).await?;
            Ok(to_addrs(ips))
        })
    }
}

fn to_addrs(ips: Vec<IpAddr>) -> Addrs {
    Box::new(ips.into_iter().map(|ip| SocketAddr::new(ip, 0)))
}

/// A recursive query for one name, id 0 as RFC 8484 suggests for caching.
pub(crate) fn encode_query(host: &str, qtype: u16) -> Result<Vec<u8>, BoxError> {
    let mut out = vec![0, 0, 0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0];
    for label in host.trim_end_matches('.').split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(format!("{host} is not a valid host name").into());
        }
        out.push(label.len() as u8);
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0);
    out.extend_from_slice(&qtype.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    Ok(out)
}

/// Addresses and the smallest time-to-live from a wire-format answer.
pub(crate) fn parse_answer(msg: &[u8]) -> Result<(Vec<IpAddr>, u32), BoxError> {
    let bad = || -> BoxError { "the DNS answer was not readable".into() };
    if msg.len() < 12 {
        return Err(bad());
    }
    let rcode = msg[3] & 0x0f;
    if rcode == 3 {
        return Ok((Vec::new(), MIN_TTL));
    }
    if rcode != 0 {
        return Err(format!("the DNS server refused the question (code {rcode})").into());
    }
    let count = |i: usize| u16::from_be_bytes([msg[i], msg[i + 1]]) as usize;
    let (questions, answers) = (count(4), count(6));
    let mut pos = 12;
    for _ in 0..questions {
        pos = skip_name(msg, pos).ok_or_else(bad)? + 4;
    }
    let mut ips = Vec::new();
    let mut ttl = u32::MAX;
    for _ in 0..answers {
        pos = skip_name(msg, pos).ok_or_else(bad)?;
        let head = msg.get(pos..pos + 10).ok_or_else(bad)?;
        let rtype = u16::from_be_bytes([head[0], head[1]]);
        let rttl = u32::from_be_bytes([head[4], head[5], head[6], head[7]]);
        let len = u16::from_be_bytes([head[8], head[9]]) as usize;
        let data = msg.get(pos + 10..pos + 10 + len).ok_or_else(bad)?;
        match (rtype, len) {
            (TYPE_A, 4) => ips.push(IpAddr::V4(Ipv4Addr::new(data[0], data[1], data[2], data[3]))),
            (TYPE_AAAA, 16) => {
                let octets: [u8; 16] = data.try_into().map_err(|_| bad())?;
                ips.push(IpAddr::V6(Ipv6Addr::from(octets)));
            }
            _ => {}
        }
        if matches!(rtype, TYPE_A | TYPE_AAAA) {
            ttl = ttl.min(rttl);
        }
        pos += 10 + len;
    }
    Ok((ips, if ttl == u32::MAX { MIN_TTL } else { ttl }))
}

/// Position just past a name, following the format's 2-byte pointers.
fn skip_name(msg: &[u8], mut pos: usize) -> Option<usize> {
    loop {
        let len = *msg.get(pos)?;
        match len {
            0 => return Some(pos + 1),
            l if l & 0xc0 == 0xc0 => {
                msg.get(pos + 1)?;
                return Some(pos + 2);
            }
            l => pos += 1 + l as usize,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_is_wire_format() {
        let q = encode_query("a.bc", TYPE_AAAA).unwrap();
        assert_eq!(q, [0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, b'a', 2, b'b', b'c', 0, 0, 28, 0, 1]);
        assert!(encode_query("a..b", TYPE_A).is_err());
    }

    #[test]
    fn answer_with_cname_and_pointers() {
        let mut msg = encode_query("ex.com", TYPE_A).unwrap();
        msg[2] = 0x81;
        msg[3] = 0x80;
        msg[7] = 2; // two answers
        // CNAME ex.com -> pointer to offset 12, 4 bytes of junk name data.
        msg.extend_from_slice(&[0xc0, 12, 0, 5, 0, 1, 0, 0, 0, 60, 0, 2, 0xc0, 12]);
        // A record, ttl 90, 1.2.3.4
        msg.extend_from_slice(&[0xc0, 12, 0, 1, 0, 1, 0, 0, 0, 90, 0, 4, 1, 2, 3, 4]);
        let (ips, ttl) = parse_answer(&msg).unwrap();
        assert_eq!(ips, vec![IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))]);
        assert_eq!(ttl, 90);
    }

    #[test]
    fn no_such_name_and_garbage() {
        let mut msg = encode_query("nope.example", TYPE_A).unwrap();
        msg[3] = 0x83;
        assert!(parse_answer(&msg).unwrap().0.is_empty());
        assert!(parse_answer(&[0, 1, 2]).is_err());
        let mut cut = encode_query("ex.com", TYPE_A).unwrap();
        cut[7] = 1;
        cut.extend_from_slice(&[0xc0, 12, 0, 1]);
        assert!(parse_answer(&cut).is_err());
    }
}
