//! Find PS5 consoles on the local network.
//!
//! The console answers a small text packet on UDP port 9302. The reply looks
//! like an HTTP response and tells us the console name, address, state, and
//! the game that is running.
//!
//! Protocol details are based on chiaki-ng's `discovery.c` and KyberNow's
//! `discovery.rs` (both AGPL-3.0).

use serde::{Deserialize, Serialize};
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

/// UDP port a PS5 listens on for discovery and wake packets.
pub const PORT_PS5: u16 = 9302;

/// Discovery protocol version a PS5 expects.
pub const PROTOCOL_PS5: &str = "00030010";

/// First local port we try to bind. The console replies to this port.
pub const LOCAL_PORT_MIN: u16 = 9303;

/// Last local port we try before asking the OS for any free port.
pub const LOCAL_PORT_MAX: u16 = 9319;

/// What the console says about itself right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConsoleState {
    /// Console is on and ready.
    Ready,
    /// Console is in rest mode.
    Standby,
    /// Console sent a state we do not know.
    Unknown,
}

impl ConsoleState {
    /// Short text for the UI and CLI.
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::Standby => "Rest mode",
            Self::Unknown => "Unknown",
        }
    }
}

/// One console that answered a discovery packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredConsole {
    /// IPv4 address the reply came from.
    pub address: String,
    /// Ready, rest mode, or unknown.
    pub state: ConsoleState,
    /// Stable console ID (usually the MAC address as hex).
    pub host_id: String,
    /// Name the owner gave the console.
    pub name: String,
    /// Usually "PS5".
    pub host_type: String,
    /// Firmware version, for example "09060000".
    pub system_version: String,
    /// Discovery protocol version.
    pub protocol_version: String,
    /// Port used by the control connection later.
    pub request_port: u16,
    /// Game ID that is running, if any.
    pub running_app_title_id: Option<String>,
    /// Game name that is running, if any.
    pub running_app_name: Option<String>,
}

impl DiscoveredConsole {
    /// True when this looks like a PS5 rather than a PS4.
    pub fn is_ps5(&self) -> bool {
        self.protocol_version == PROTOCOL_PS5 || self.host_type.eq_ignore_ascii_case("PS5")
    }
}

/// Send discovery packets and collect replies until `wait` runs out.
///
/// Broadcast packets go to every IPv4 network interface, plus the global
/// broadcast address. `direct` lets the caller also try known addresses, which
/// helps on networks that block broadcast.
pub fn search(wait: Duration, direct: &[IpAddr]) -> io::Result<Vec<DiscoveredConsole>> {
    let socket = open_socket()?;
    socket.set_broadcast(true)?;

    let targets = packet_targets(direct);
    let deadline = Instant::now() + wait;
    let mut next_send = Instant::now();
    let mut found: Vec<DiscoveredConsole> = Vec::new();
    let mut buf = [0u8; 2048];

    loop {
        let now = Instant::now();
        if now >= deadline {
            break;
        }

        if now >= next_send {
            for ip in &targets {
                // A failed send on one interface should not stop the scan.
                let _ = socket.send_to(&search_packet(), SocketAddr::new(*ip, PORT_PS5));
            }
            next_send = now + Duration::from_millis(500);
        }

        let next_wake = deadline.min(next_send);
        let timeout = next_wake
            .saturating_duration_since(Instant::now())
            .max(Duration::from_millis(1));
        socket.set_read_timeout(Some(timeout))?;

        match socket.recv_from(&mut buf) {
            Ok((len, SocketAddr::V4(from))) => {
                if let Some(console) = parse_response(*from.ip(), &buf[..len]) {
                    if let Some(existing) = found.iter_mut().find(|c| same_console(c, &console)) {
                        *existing = console;
                    } else {
                        tracing::info!(
                            name = %console.name,
                            address = %console.address,
                            state = ?console.state,
                            "found console"
                        );
                        found.push(console);
                    }
                }
            }
            Ok(_) => {}
            // Windows reports an ICMP "port closed" as ConnectionReset on a
            // UDP socket. That just means one address had nothing listening.
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::ConnectionReset
                        | io::ErrorKind::ConnectionAborted
                        | io::ErrorKind::Interrupted
                ) => {}
            Err(e) => return Err(e),
        }
    }

    found.sort_by_key(|console| console.name.to_lowercase());
    Ok(found)
}

/// The discovery packet, including the trailing zero byte used by chiaki-ng.
pub fn search_packet() -> Vec<u8> {
    let mut packet =
        format!("SRCH * HTTP/1.1\ndevice-discovery-protocol-version:{PROTOCOL_PS5}\n").into_bytes();
    packet.push(0);
    packet
}

/// Read one reply from a console. Returns `None` when the data is not a
/// PlayStation discovery response.
pub fn parse_response(addr: Ipv4Addr, data: &[u8]) -> Option<DiscoveredConsole> {
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    let text = String::from_utf8_lossy(&data[..end]);
    let mut lines = text.split('\n').map(|line| line.trim_end_matches('\r'));

    let status = lines.next()?;
    let mut status_parts = status.splitn(3, ' ');
    if !status_parts.next()?.starts_with("HTTP/") {
        return None;
    }
    let code: u16 = status_parts.next()?.trim().parse().ok()?;
    let state = match code {
        200 => ConsoleState::Ready,
        620 => ConsoleState::Standby,
        _ => ConsoleState::Unknown,
    };

    let mut console = DiscoveredConsole {
        address: addr.to_string(),
        state,
        host_id: String::new(),
        name: String::new(),
        host_type: String::new(),
        system_version: String::new(),
        protocol_version: String::new(),
        request_port: 0,
        running_app_title_id: None,
        running_app_name: None,
    };

    for line in lines {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().to_string();
        match key.trim() {
            "host-id" => console.host_id = value,
            "host-name" => console.name = value,
            "host-type" => console.host_type = value,
            "system-version" => console.system_version = value,
            "device-discovery-protocol-version" => console.protocol_version = value,
            "host-request-port" => console.request_port = value.parse().unwrap_or(0),
            "running-app-titleid" if !value.is_empty() => {
                console.running_app_title_id = Some(value)
            }
            "running-app-name" if !value.is_empty() => console.running_app_name = Some(value),
            _ => {}
        }
    }

    if console.name.is_empty() {
        console.name = "PlayStation 5".to_string();
    }

    Some(console)
}

fn open_socket() -> io::Result<UdpSocket> {
    for port in LOCAL_PORT_MIN..=LOCAL_PORT_MAX {
        if let Ok(socket) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port)) {
            return Ok(socket);
        }
    }
    UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
}

fn packet_targets(direct: &[IpAddr]) -> Vec<IpAddr> {
    let mut targets = vec![IpAddr::V4(Ipv4Addr::BROADCAST)];

    if let Ok(interfaces) = if_addrs::get_if_addrs() {
        for interface in interfaces {
            let if_addrs::IfAddr::V4(v4) = interface.addr else {
                continue;
            };
            if v4.ip.is_loopback() {
                continue;
            }
            if let Some(broadcast) = v4.broadcast {
                let ip = IpAddr::V4(broadcast);
                if !targets.contains(&ip) {
                    targets.push(ip);
                }
            }
        }
    }

    for ip in direct {
        if !targets.contains(ip) {
            targets.push(*ip);
        }
    }

    targets
}

fn same_console(a: &DiscoveredConsole, b: &DiscoveredConsole) -> bool {
    if !a.host_id.is_empty() && !b.host_id.is_empty() {
        a.host_id == b.host_id
    } else {
        a.address == b.address
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_packet_matches_chiaki() {
        assert_eq!(
            search_packet(),
            b"SRCH * HTTP/1.1\ndevice-discovery-protocol-version:00030010\n\0".to_vec()
        );
    }

    #[test]
    fn parses_ps5_in_rest_mode() {
        let reply = b"HTTP/1.1 620 Server Standby\n\
            host-id:A1B2C3D4E5F6\n\
            host-type:PS5\n\
            host-name:Living Room\n\
            host-request-port:997\n\
            device-discovery-protocol-version:00030010\n\
            system-version:09060000\n\0";
        let console = parse_response(Ipv4Addr::new(192, 168, 1, 50), reply).unwrap();
        assert_eq!(console.state, ConsoleState::Standby);
        assert_eq!(console.name, "Living Room");
        assert_eq!(console.address, "192.168.1.50");
        assert_eq!(console.request_port, 997);
        assert!(console.is_ps5());
    }

    #[test]
    fn parses_running_game() {
        let reply = b"HTTP/1.1 200 Ok\r\n\
            host-id:001122334455\r\n\
            host-type:PS5\r\n\
            host-name:Den\r\n\
            host-request-port:997\r\n\
            device-discovery-protocol-version:00030010\r\n\
            system-version:10000000\r\n\
            running-app-titleid:PPSA00001\r\n\
            running-app-name:Game: The Sequel\r\n";
        let console = parse_response(Ipv4Addr::new(10, 0, 0, 2), reply).unwrap();
        assert_eq!(console.state, ConsoleState::Ready);
        assert_eq!(console.running_app_title_id.as_deref(), Some("PPSA00001"));
        assert_eq!(
            console.running_app_name.as_deref(),
            Some("Game: The Sequel")
        );
    }

    #[test]
    fn rejects_other_packets() {
        assert_eq!(
            parse_response(Ipv4Addr::LOCALHOST, b"SRCH * HTTP/1.1\n"),
            None
        );
        assert_eq!(parse_response(Ipv4Addr::LOCALHOST, b"garbage"), None);
    }

    #[test]
    fn dedupes_by_host_id() {
        let a = DiscoveredConsole {
            address: "192.168.1.10".into(),
            state: ConsoleState::Ready,
            host_id: "ABC".into(),
            name: "A".into(),
            host_type: "PS5".into(),
            system_version: String::new(),
            protocol_version: PROTOCOL_PS5.into(),
            request_port: 997,
            running_app_title_id: None,
            running_app_name: None,
        };
        let mut b = a.clone();
        b.address = "192.168.1.11".into();
        assert!(same_console(&a, &b));
    }
}
