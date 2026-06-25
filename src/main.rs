mod blocklist;
mod server;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Result;
use tokio::{net::UdpSocket, sync::Mutex, time::timeout};

use tracing::{error, info, warn};
use crate::blocklist::Blocklist;
use crate::server::Server;

#[derive(Debug)]
struct Header {
    id: u16,
    /// Query(true) or Response(false)
    qr: bool,
    opcode: u8, // 4 bits
    aa: bool,
    tc: bool,
    rd: bool,
    ra: bool,
    z: u8, // 3 bits, reserved for future use
    rcode: u8,
    qdcount: u16,
    ancount: u16,
    nscount: u16,
    arcount: u16,
}

#[derive(Debug)]
struct Question {
    qname: String, // Each label has a u8 for length of label. 0x003 0x777777 = www
    qtype: u16,
    qclass: u16,
}

#[derive(Debug)]
struct Answer {
    name: String, // Each label has a u8 for length of label. 0x003 0x777777 = www
    r#type: u16,
    class: u16,
    ttl: u32,
    rdlength: u16,
    rdata: Vec<u8>, // depends on type field
}

#[derive(Debug)]
struct Request {
    header: Header,
    question: Question,
}

fn parse_dns_request(bytes: &[u8]) -> Result<Request> {
    let id = u16::from_be_bytes(bytes[0..2].try_into()?);

    let flags = bytes[2];
    let qr = (flags >> 7) & 1 != 0;
    let opcode = (flags >> 3) & 0x0F;
    let aa = (flags >> 2) & 1 != 0;
    let tc = (flags >> 1) & 1 != 0;
    let rd = flags & 1 != 0;
    let ra = (bytes[3] >> 7) & 1 != 0;

    let z = (bytes[3] >> 4) & 0x07;
    let rcode = bytes[3] & 0x0F;

    let header = Header {
        id,
        qr,
        opcode,
        aa,
        tc,
        rd,
        ra,
        z,
        rcode,
        qdcount: u16::from_be_bytes(bytes[4..=5].try_into()?),
        ancount: u16::from_be_bytes(bytes[6..=7].try_into()?),
        nscount: u16::from_be_bytes(bytes[8..=9].try_into()?),
        arcount: u16::from_be_bytes(bytes[10..=11].try_into()?),
    };

    let mut qname = String::new();
    let mut pos = 12;
    let mut next_chunk: usize = bytes[pos] as usize;

    while next_chunk != 0 {
        if !qname.is_empty() {
            qname.push('.');
        }
        qname.push_str(std::str::from_utf8(&bytes[pos + 1..pos + 1 + next_chunk])?);
        pos += next_chunk + 1;
        next_chunk = bytes[pos] as usize;
    }

    pos += 1;

    let question = Question {
        qname,
        qtype: u16::from_be_bytes(bytes[pos..=pos + 1].try_into()?),
        qclass: u16::from_be_bytes(bytes[pos + 2..=pos + 3].try_into()?),
    };

    Ok(Request { header, question })
}

fn build_blocked_answer(request_bytes: &[u8], parsed: &Request) -> Vec<u8> {
    let (ancount, rcode): (u16, u8) = match parsed.question.qtype {
        1 | 28 => (1, 0),
        _ => (0, 3), // NXDOMAIN for record types we can't synthesize
    };

    let mut resp = Vec::with_capacity(64);

    // Header
    resp.extend_from_slice(&parsed.header.id.to_be_bytes());
    // QR=1, Opcode=0, AA=1, TC=0, RD=request.rd
    resp.push(0b1000_0100u8 | u8::from(parsed.header.rd));
    resp.push(rcode); // RA=0, Z=0, RCODE
    resp.extend_from_slice(&1u16.to_be_bytes()); // QDCOUNT
    resp.extend_from_slice(&ancount.to_be_bytes()); // ANCOUNT
    resp.extend_from_slice(&0u16.to_be_bytes()); // NSCOUNT
    resp.extend_from_slice(&0u16.to_be_bytes()); // ARCOUNT

    // Question section: find the end of the wire-format name then copy qtype+qclass
    let mut qend = 12;
    while qend < request_bytes.len() && request_bytes[qend] != 0 {
        qend += request_bytes[qend] as usize + 1;
    }
    qend += 5; // null terminator (1) + qtype (2) + qclass (2)
    resp.extend_from_slice(&request_bytes[12..qend]);

    // Answer section
    if ancount == 1 {
        resp.extend_from_slice(&0xC00Cu16.to_be_bytes()); // name pointer to offset 12
        match parsed.question.qtype {
            1 => {
                resp.extend_from_slice(&1u16.to_be_bytes()); // TYPE A
                resp.extend_from_slice(&1u16.to_be_bytes()); // CLASS IN
                resp.extend_from_slice(&60u32.to_be_bytes()); // TTL
                resp.extend_from_slice(&4u16.to_be_bytes()); // RDLENGTH
                resp.extend_from_slice(&[0u8; 4]); // 0.0.0.0
            }
            28 => {
                resp.extend_from_slice(&28u16.to_be_bytes()); // TYPE AAAA
                resp.extend_from_slice(&1u16.to_be_bytes()); // CLASS IN
                resp.extend_from_slice(&60u32.to_be_bytes()); // TTL
                resp.extend_from_slice(&16u16.to_be_bytes()); // RDLENGTH
                resp.extend_from_slice(&[0u8; 16]); // ::
            }
            _ => unreachable!(),
        }
    }

    resp
}

async fn forward_to_upstream(bytes: &[u8]) -> Result<Vec<u8>> {
    for server in ["8.8.8.8:53", "1.1.1.1:53"] {
        let upstream = UdpSocket::bind("0.0.0.0:0").await?;
        upstream.connect(server).await?;
        upstream.send(bytes).await?;
        let mut buf = vec![0u8; 4096];
        match timeout(Duration::from_secs(3), upstream.recv(&mut buf)).await {
            Ok(Ok(n)) => {
                buf.truncate(n);
                return Ok(buf);
            }
            _ => continue,
        }
    }
    anyhow::bail!("all upstream servers timed out")
}

fn build_servfail(request_bytes: &[u8], id: u16) -> Vec<u8> {
    let mut resp = Vec::with_capacity(64);
    resp.extend_from_slice(&id.to_be_bytes());
    resp.push(0x80); // QR=1
    resp.push(0x02); // RCODE=SERVFAIL
    resp.extend_from_slice(&1u16.to_be_bytes()); // QDCOUNT
    resp.extend_from_slice(&0u16.to_be_bytes()); // ANCOUNT
    resp.extend_from_slice(&0u16.to_be_bytes()); // NSCOUNT
    resp.extend_from_slice(&0u16.to_be_bytes()); // ARCOUNT
    // Echo the question section so clients can match the response
    let mut qend = 12;
    while qend < request_bytes.len() && request_bytes[qend] != 0 {
        qend += request_bytes[qend] as usize + 1;
    }
    if qend + 5 <= request_bytes.len() {
        resp.extend_from_slice(&request_bytes[12..qend + 5]);
    }
    resp
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let socket = Arc::new(UdpSocket::bind("0.0.0.0:53".parse::<SocketAddr>()?).await?);
    let blocklist = Arc::new(Mutex::new(Blocklist::new()));

    tokio::spawn(Blocklist::spawn(blocklist.clone()));
    
    tokio::spawn(Server::run(blocklist.clone()));

    let mut buf = [0u8; 1024];
    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;
        let bytes = buf[..len].to_vec();
        let socket = socket.clone();
        let blocklist = blocklist.clone();

        tokio::spawn(async move {
            let Ok(parsed) = parse_dns_request(&bytes) else {
                warn!("failed to parse DNS request");
                return;
            };

            let response = if blocklist.lock().await.check(&parsed.question.qname) {
                info!(domain = %parsed.question.qname, "blocked");
                build_blocked_answer(&bytes, &parsed)
            } else {
                match forward_to_upstream(&bytes).await {
                    Ok(resp) => resp,
                    Err(e) => {
                        error!(domain = %parsed.question.qname, error = %e, "upstream error");
                        build_servfail(&bytes, parsed.header.id)
                    }
                }
            };

            if let Err(e) = socket.send_to(&response, addr).await {
                error!(error = %e, "send error");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_dns_request() {
        let bytes = vec![
            198, 8, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 3, 119, 119, 119, 6, 103, 111, 111, 103, 108,
            101, 3, 99, 111, 109, 0, 0, 1, 0, 1, 3, 99, 111, 109, 0, 0, 28, 0, 1, 3, 99, 111, 109,
            0, 0, 1, 0, 1, 108, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let result = crate::parse_dns_request(bytes.as_slice()).unwrap();

        println!("{:#?}", result);
    }
}
