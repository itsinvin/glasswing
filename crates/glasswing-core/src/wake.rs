//! Wake a PS5 from rest mode.
//!
//! The wake packet needs the console's pairing key. Without a key the packet
//! is still valid, but the console will ignore it. That is why wake only works
//! after pairing.
//!
//! Protocol details are based on chiaki-ng's `discovery.c` (AGPL-3.0).

use crate::discovery::{PORT_PS5, PROTOCOL_PS5};
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};

/// Build the wake packet, including the trailing zero byte.
pub fn wake_packet(credential: u64) -> Vec<u8> {
    let mut packet = format!(
        "WAKEUP * HTTP/1.1\n\
         client-type:vr\n\
         auth-type:R\n\
         model:w\n\
         app-type:r\n\
         user-credential:{credential}\n\
         device-discovery-protocol-version:{PROTOCOL_PS5}\n"
    )
    .into_bytes();
    packet.push(0);
    packet
}

/// Send one wake packet to a console.
///
/// `address` can be an IPv4 address, a host name, or `host:port`. Port 9302 is
/// used when no port is given.
pub fn wake(address: &str, credential: u64) -> io::Result<()> {
    let target = resolve_target(address)?;
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    socket.set_broadcast(true)?;
    socket.send_to(&wake_packet(credential), target)?;
    Ok(())
}

/// Read a pairing key from hex text.
///
/// Empty text means "no key" and returns 0. This keeps the UI simple before
/// pairing exists.
pub fn credential_from_hex(text: &str) -> Result<u64, String> {
    let cleaned: String = text
        .trim()
        .trim_start_matches("0x")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    if cleaned.is_empty() {
        return Ok(0);
    }
    if cleaned.len() > 16 {
        return Err("Pairing key must be 16 hex characters or less.".to_string());
    }

    u64::from_str_radix(&cleaned, 16)
        .map_err(|_| "Pairing key can only use the characters 0-9 and a-f.".to_string())
}

fn resolve_target(address: &str) -> io::Result<SocketAddr> {
    let text = address.trim();
    if text.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "console address is empty",
        ));
    }

    if let Ok(socket_addr) = text.parse::<SocketAddr>() {
        return Ok(socket_addr);
    }
    if let Ok(ip) = text.parse::<IpAddr>() {
        return Ok(SocketAddr::new(ip, PORT_PS5));
    }

    let with_port = format!("{text}:{PORT_PS5}");
    with_port
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::AddrNotAvailable, "address not found"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_packet_matches_chiaki() {
        assert_eq!(
            wake_packet(0x1234_5678),
            b"WAKEUP * HTTP/1.1\nclient-type:vr\nauth-type:R\nmodel:w\napp-type:r\n\
              user-credential:305419896\ndevice-discovery-protocol-version:00030010\n\0"
                .to_vec()
        );
    }

    #[test]
    fn reads_hex_key() {
        assert_eq!(credential_from_hex("12345678").unwrap(), 0x1234_5678);
        assert_eq!(credential_from_hex("0xABCDEF01").unwrap(), 0xABCD_EF01);
        assert_eq!(credential_from_hex("  ").unwrap(), 0);
        assert!(credential_from_hex("not-hex").is_err());
        assert!(credential_from_hex("12345678901234567").is_err());
    }

    #[test]
    fn resolves_plain_ipv4() {
        let target = resolve_target("192.168.1.50").unwrap();
        assert_eq!(target, SocketAddr::from(([192, 168, 1, 50], PORT_PS5)));
    }
}
