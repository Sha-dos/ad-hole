use std::{net::SocketAddr, sync::Arc};

use tokio::{net::UdpSocket, sync::mpsc};

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

struct Question {
    qname: String, // Each label has a u8 for length of label. 0x003 0x777777 = www
    qtype: u16,
    qclass: u16,
}

struct Answer {
    name: String, // Each label has a u8 for length of label. 0x003 0x777777 = www
    r#type: u16,
    class: u16,
    ttl: u32,
    rdlength: u16,
    rdata: Vec<u8>, // depends on type field
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:53".parse::<SocketAddr>()?).await?;
    let r = Arc::new(socket);
    let s = r.clone();

    let (tx, mut rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);

    tokio::spawn(async move {
        while let Some((bytes, addr)) = rx.recv().await {
            let len = s.send_to(&bytes, &addr).await.unwrap();
            println!("{:?} bytes sent", len);
        }
    });

    let mut buf = [0; 1024];
    loop {
        let (len, addr) = r.recv_from(&mut buf).await?;

        println!("{:?} bytes received from {:?}", len, addr);
        println!("raw: {:?}", buf);
        println!("utf8 lossy: {}", String::from_utf8_lossy(&buf));

        tx.send((buf[..len].to_vec(), addr)).await.unwrap();
    }

    Ok(())
}
