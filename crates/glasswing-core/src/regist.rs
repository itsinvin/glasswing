//! Pairing with a PS5, which Sony's protocol calls a "regist".
//!
//! The console runs a small UDP/TCP server on port 9295. To pair, the user opens
//! Settings > Linked Devices > Link Device on the console, types the PIN it shows
//! into Glasswing, and Glasswing exchanges four short messages. The console
//! answers with the regist key that later sessions and wake packets use.
//!
//! Two UDP packets open the door (`SRC3`, then `RES3` from the console), then a
//! TCP connection carries an HTTP POST whose body is encrypted with the RP
//! cipher in [`crate::rpcrypt`]. The response body, also encrypted, carries the
//! key back.
//!
//! Ported from chiaki-ng (AGPL-3.0), `lib/src/regist.c` and `lib/src/rpcrypt.c`.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::time::{Duration, Instant};

use crate::rpcrypt::{self, RPCrypt, Target};

/// The console's regist port, UDP for the search and TCP for the request.
pub const REGIST_PORT: u16 = 9295;

/// How long to wait for the console to answer the search.
const SEARCH_TIMEOUT: Duration = Duration::from_millis(3_000);
/// How long to wait for the regist response.
const RESPONSE_TIMEOUT: Duration = Duration::from_millis(3_000);
/// The console will not take a request immediately after the search (chiaki waits the same).
const SETTLE: Duration = Duration::from_millis(100);

/// What the client sends to ask "is the Link Device screen open?"
const SEARCH_PACKET: &[u8] = b"SRC3\0";
/// What the console answers when it is.
const SEARCH_REPLY: &[u8] = b"RES3";

/// The path the regist request is POSTed to.
const REQUEST_PATH: &str = "/sie/ps5/rp/sess/rgst";
/// The Remote Play protocol version Glasswing speaks. PS5 is 1.0.
const RP_VERSION: &str = "1.0";
/// Identifies this app to the console, as the official Windows client does.
const CLIENT_TYPE: &str = "dabfa2ec873de5839bee8d3f4c0239c4282c07c25c6077a2931afcf0adc0d34f";
/// The local address the request claims, when the real one is not known yet.
const LOCAL_ADDR: &str = "10.0.2.15";
/// Where the encrypted inner header starts in the payload, and the payload's size.
const INNER_HEADER_OFFSET: usize = 0x1e0;
const PAYLOAD_SIZE: usize = 0x400;
/// A PSN account ID is 8 bytes, and base64 of it is 12 characters.
const ACCOUNT_ID_SIZE: usize = 8;

/// Which console a key is for. Only PS5 today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Console {
    /// A PS5 that answers RP-Version 1.0.
    Ps5,
}

impl Console {
    /// The RP-Version string this client sends.
    pub fn rp_version(self) -> &'static str {
        match self {
            Console::Ps5 => RP_VERSION,
        }
    }

    /// The RPCrypt target for this console.
    pub fn rpcrypt_target(self) -> Target {
        match self {
            Console::Ps5 => Target::Ps5_1,
        }
    }
}

/// What pairing needs before it can start.
pub struct PairingRequest<'a> {
    /// The console's address, as an IPv4 address or a name.
    pub address: &'a str,
    /// The port to talk on, so tests can use a free one.
    pub port: u16,
    /// Which console this is.
    pub console: Console,
    /// The 8-byte PSN account ID.
    pub account_id: [u8; ACCOUNT_ID_SIZE],
    /// The PIN the console shows on its Link Device screen.
    pub pin: u32,
    /// Called as pairing moves along, for a window to show where it is.
    pub on_step: Option<Box<dyn Fn(PairingStep) + Send>>,
}

/// Why pairing stopped short.
#[derive(Debug, thiserror::Error)]
pub enum RegistError {
    /// The console did not answer the search.
    #[error("the console did not answer. Is its Link Device (or Add Device) screen open?")]
    NoAnswer,
    /// The console answered the search but would not take the request.
    #[error("the console refused the request. Keep its Link Device screen open while pairing.")]
    Refused,
    /// Nothing came back in time.
    #[error("the console stopped answering")]
    Timeout,
    /// A socket failed.
    #[error("network error: {0}")]
    Network(String),
    /// The console answered something we cannot read.
    #[error("the console's answer made no sense: {0}")]
    BadResponse(String),
    /// The cipher refused the data.
    #[error("the reply could not be decrypted: {0}")]
    Crypt(String),
}

/// The steps pairing moves through, for a window to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairingStep {
    /// Sending the search packet.
    Searching,
    /// The console answered; opening the request connection.
    Connecting,
    /// The request is on its way.
    Waiting,
    /// The answer is in; reading the key.
    Finishing,
}

impl PairingStep {
    /// A short line for a person.
    pub fn text(self) -> &'static str {
        match self {
            PairingStep::Searching => "Looking for the console's pairing screenâ€¦",
            PairingStep::Connecting => "The console answered. Opening the pairing connectionâ€¦",
            PairingStep::Waiting => "Waiting for the console to finish pairingâ€¦",
            PairingStep::Finishing => "Reading the keyâ€¦",
        }
    }
}

/// Everything the console told us about itself, and the key to keep.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairedHost {
    /// The name the owner gave the console.
    pub nickname: String,
    /// The console's MAC address, which later sessions compare.
    pub mac: [u8; 6],
    /// The key this pairing bought: sessions and wake packets both use it.
    pub regist_key: [u8; 0x10],
    /// A second, shorter key.
    pub rp_key: [u8; 0x10],
    /// What kind of key `rp_key` is.
    pub rp_key_type: u32,
    /// The wireless network the console is on, when it tells us.
    pub ap_ssid: String,
    pub ap_bssid: String,
    pub ap_key: String,
    pub ap_name: String,
    /// The PIN the user typed, so the app can confirm what it paired as.
    pub console_pin: u32,
}

impl PairedHost {
    /// The regist key as lower-case hex, the way chiaki and this app save it.
    pub fn regist_key_hex(&self) -> String {
        hex(&self.regist_key)
    }
}

/// Read the PSN account ID a person pastes into Settings, usually base64 of 8
/// bytes. A hex string or raw text of 8 characters is also accepted, because
/// people paste whatever their source gave them.
pub fn decode_account_id(text: &str) -> Result<[u8; ACCOUNT_ID_SIZE], String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("no account ID is saved".to_string());
    }
    // Base64 comes first, because that is what the console's pairing screen
    // tells a person to copy. Hex and plain text are accepted too, since what
    // a person actually has depends on where they read it.
    let mut bytes = Vec::new();
    if let Ok(raw) = base64_decode(text) {
        if raw.len() == ACCOUNT_ID_SIZE {
            bytes = raw;
        }
    }
    if bytes.is_empty() && text.len() == ACCOUNT_ID_SIZE * 2 && is_hex(text) {
        for pair in text.as_bytes().chunks(2) {
            let digits = std::str::from_utf8(pair).unwrap_or("00");
            bytes.push(u8::from_str_radix(digits, 16).unwrap_or(0));
        }
    }
    if bytes.is_empty() && text.len() == ACCOUNT_ID_SIZE {
        bytes = text.as_bytes().to_vec();
    }
    if bytes.is_empty() {
        return Err(
            "the account ID should be base64 of 8 bytes, 16 hex characters, or 8 bytes of text"
                .to_string(),
        );
    }
    bytes
        .try_into()
        .map_err(|_| "the account ID is not 8 bytes".to_string())
}

/// Whether every byte is a hex digit.
fn is_hex(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Base64 without a crate.
fn base64_decode(text: &str) -> Result<Vec<u8>, ()> {
    fn value(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::new();
    let mut chunk = 0u32;
    let mut bits = 0u32;
    for byte in text.bytes() {
        if byte == b'=' {
            break;
        }
        let value = value(byte).ok_or(())?;
        chunk = (chunk << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((chunk >> bits) as u8);
        }
    }
    Ok(out)
}

/// Pair with a console. Blocks until it succeeds or fails, so run it on a worker
/// thread, never on the UI thread.
pub fn pair(request: &PairingRequest<'_>) -> Result<PairedHost, RegistError> {
    let step = |s: PairingStep| {
        if let Some(on_step) = &request.on_step {
            on_step(s);
        }
    };

    step(PairingStep::Searching);
    let peer = search(request)?;

    step(PairingStep::Connecting);
    std::thread::sleep(SETTLE);
    let mut stream = TcpStream::connect_timeout(&peer, RESPONSE_TIMEOUT)
        .map_err(|error| RegistError::Network(error.to_string()))?;
    stream
        .set_read_timeout(Some(RESPONSE_TIMEOUT))
        .map_err(|error| RegistError::Network(error.to_string()))?;
    stream
        .set_write_timeout(Some(RESPONSE_TIMEOUT))
        .map_err(|error| RegistError::Network(error.to_string()))?;

    step(PairingStep::Waiting);
    let ambassador = random_ambassador();
    let request_bytes = build_request(request, &ambassador);
    stream
        .write_all(&request_bytes)
        .map_err(|error| RegistError::Network(error.to_string()))?;

    let response = read_response(&mut stream)?;

    step(PairingStep::Finishing);
    let crypt = RPCrypt::init_regist(
        request.console.rpcrypt_target(),
        &ambassador,
        usize::from(b'A' & 0x1F),
        request.pin,
    )
    .map_err(|error| RegistError::Crypt(error.to_string()))?;
    let mut payload = response.payload;
    let mut decrypted = payload.clone();
    crypt
        .decrypt(0, &payload, &mut decrypted)
        .map_err(|error| RegistError::Crypt(error.to_string()))?;
    payload = decrypted;

    let mut host = parse_response_payload(&payload)?;
    host.console_pin = request.pin;
    Ok(host)
}

/// Ask the console whether its pairing screen is open, and where to send the request.
///
/// The console only answers while the user is on the Link Device screen, so a
/// quiet console here usually means the screen is closed rather than that the
/// console is gone.
fn search(request: &PairingRequest<'_>) -> Result<SocketAddr, RegistError> {
    let target: SocketAddr = (peer_ip(request.address)?, request.port).into();
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(net)?;
    socket.connect(target).map_err(net)?;
    socket
        .send(SEARCH_PACKET)
        .map_err(|error| network(&error))?;

    let mut buf = [0u8; 0x100];
    let deadline = Instant::now() + SEARCH_TIMEOUT;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(RegistError::Timeout);
        }
        socket.set_read_timeout(Some(left)).map_err(net)?;
        match socket.recv_from(&mut buf) {
            Ok((n, from)) => {
                if n >= SEARCH_REPLY.len() && buf[..SEARCH_REPLY.len()] == *SEARCH_REPLY {
                    return Ok(from);
                }
                // Some other UDP chatter; keep listening until the deadline.
            }
            Err(error) => {
                if is_would_block(&error) {
                    continue;
                }
                // Windows reports a closed port as a reset, which here means the
                // console turned the search away: its pairing screen is closed.
                if is_refused(&error) {
                    return Err(RegistError::Refused);
                }
                return Err(network(&error));
            }
        }
    }
}

/// The whole request: an HTTP header, then the encrypted payload.
fn build_request(request: &PairingRequest<'_>, ambassador: &[u8; 0x10]) -> Vec<u8> {
    let payload = build_payload(request, ambassador);
    let local = local_addr(request.address);
    let header = format!(
        "POST {REQUEST_PATH} HTTP/1.1\r\n HTTP/1.1\r\n\
         HOST: {local}\r\n\
         User-Agent: remoteplay Windows\r\n\
         Connection: close\r\n\
         Content-Length: {}\r\n\
         RP-Version: {}\r\n\r\n",
        payload.len(),
        request.console.rp_version(),
    );
    let mut out = header.into_bytes();
    out.extend_from_slice(&payload);
    out
}

/// The request body: padding, the two cipher nonces, then the encrypted account
/// ID header the console uses to know who is pairing.
fn build_payload(request: &PairingRequest<'_>, ambassador: &[u8; 0x10]) -> Vec<u8> {
    let mut payload = vec![b'A'; PAYLOAD_SIZE];

    // The two nonce positions come out of the padding itself, so a real console
    // and this client agree without any handshake.
    let key_0_off = usize::from(payload[0x18D] & 0x1F);
    let key_1_off = usize::from(payload[0] >> 3);

    let target = request.console.rpcrypt_target();
    let crypt = RPCrypt::init_regist(target, ambassador, key_0_off, request.pin)
        .expect("a regist key for a known console");
    let mut aeropause = [0u8; 0x10];
    rpcrypt::aeropause(target, key_1_off, &mut aeropause, ambassador)
        .expect("an aeropause for a known console");
    // chiaki writes eight bytes at each offset, not the whole sixteen.
    payload[0xc7..0xcf].copy_from_slice(&aeropause[8..]);
    payload[0x191..0x199].copy_from_slice(&aeropause[..8]);

    let inner = format!(
        "Client-Type: {CLIENT_TYPE}\r\nNp-AccountId: {}\r\n",
        base64_encode(&request.account_id)
    );
    let plain = inner.into_bytes();
    let mut sealed = vec![0u8; plain.len()];
    payload.truncate(INNER_HEADER_OFFSET);
    payload.extend_from_slice(&plain);
    crypt
        .encrypt(0, &plain, &mut sealed)
        .expect("to encrypt our own request");
    payload[INNER_HEADER_OFFSET..].copy_from_slice(&sealed);
    payload
}

/// The console's answer: the payload it carries.
struct Response {
    payload: Vec<u8>,
}

/// Read the response: the HTTP header, then as many payload bytes as it promises.
fn read_response(stream: &mut TcpStream) -> Result<Response, RegistError> {
    let mut buf: Vec<u8> = Vec::with_capacity(1500);
    let mut scratch = [0u8; 1500];

    // Read until the header's blank line.
    let header_end = loop {
        if let Some(end) = find_subslice(&buf, b"\r\n\r\n") {
            break end;
        }
        if buf.len() >= 0x1000 {
            return Err(RegistError::BadResponse(
                "the HTTP header was too long".into(),
            ));
        }
        let want = scratch.len().min(0x1000);
        let read = stream.read(&mut scratch[..want]).map_err(|error| {
            if error.kind() == std::io::ErrorKind::WouldBlock
                || error.kind() == std::io::ErrorKind::TimedOut
            {
                RegistError::Timeout
            } else {
                network(&error)
            }
        })?;
        if read == 0 {
            return Err(RegistError::BadResponse("the console closed early".into()));
        }
        buf.extend_from_slice(&scratch[..read]);
    };

    let header = buf[..header_end + 4].to_vec();
    let rest = buf.len() - header.len();
    let (status, content_length) = parse_response_header(&header)?;
    if status != 200 {
        let reason = headers_of(&header)
            .into_iter()
            .find(|(key, _)| key == "RP-Application-Reason")
            .map(|(_, value)| value)
            .unwrap_or_default();
        return Err(RegistError::BadResponse(format!(
            "the console said HTTP {status} (reason {reason})",
        )));
    }
    let content_length = content_length
        .ok_or_else(|| RegistError::BadResponse("the answer had no Content-Length".to_string()))?;

    let mut payload = buf[buf.len() - rest..].to_vec();
    while payload.len() < content_length {
        let want = scratch.len().min(content_length - payload.len());
        let read = stream.read(&mut scratch[..want]).map_err(|error| {
            if error.kind() == std::io::ErrorKind::WouldBlock
                || error.kind() == std::io::ErrorKind::TimedOut
            {
                RegistError::Timeout
            } else {
                network(&error)
            }
        })?;
        if read == 0 {
            return Err(RegistError::BadResponse("the console closed early".into()));
        }
        payload.extend_from_slice(&scratch[..read]);
    }
    payload.truncate(content_length);
    Ok(Response { payload })
}

/// The status code and the Content-Length of a response header.
fn parse_response_header(header: &[u8]) -> Result<(u16, Option<usize>), RegistError> {
    let text = String::from_utf8(header.to_vec())
        .map_err(|_| RegistError::BadResponse("bad header".into()))?;
    let first = text
        .lines()
        .next()
        .ok_or_else(|| RegistError::BadResponse("an empty header".into()))?;
    let mut parts = first.split_whitespace();
    let _version = parts.next();
    let code = parts
        .next()
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| RegistError::BadResponse("no status code".into()))?;
    let length = headers_of(header)
        .into_iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("Content-Length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok());
    Ok((code, length))
}

/// The console's address as an IP. Names are not resolved yet: discovery gives
/// us an address, and a manual entry is typed as one.
fn peer_ip(address: &str) -> Result<std::net::IpAddr, RegistError> {
    address
        .trim()
        .parse()
        .map_err(|_| RegistError::Network(format!("no address in {address:?}")))
}

/// The local address the request claims. chiaki sends a static one here, and the
/// console does not check it, so neither do we.
fn local_addr(_address: &str) -> &'static str {
    LOCAL_ADDR
}

/// The first index of `needle` in `haystack`.
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// A 16-byte key for this one pairing attempt.
///
/// The ambassador is a throwaway: the console's answer is what makes it useful,
/// and the key that answer carries is what lasts. `RandomState` gets its seed
/// from the OS, and hashing a counter through it mixes in some of that seed each
/// time, which is plenty for a value that lives for a few seconds.
fn random_ambassador() -> [u8; 0x10] {
    use std::hash::{BuildHasher, RandomState};

    let mut out = [0u8; 0x10];
    let mut hasher = RandomState::new().build_hasher();
    let mut counter = std::time::Instant::now().elapsed().as_nanos() as u64;
    for slot in out.iter_mut() {
        counter = counter.wrapping_add(1);
        std::hash::Hasher::write_u64(&mut hasher, counter);
        std::hash::Hasher::write_u8(&mut hasher, *slot ^ 0x5a);
        *slot = (std::hash::Hasher::finish(&hasher) & 0xff) as u8;
    }
    out
}

/// Whether a read timed out.
fn is_would_block(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    )
}

/// Whether the peer turned us away, which Windows reports as a reset.
fn is_refused(error: &std::io::Error) -> bool {
    if cfg!(windows) {
        // WSAECONNRESET, from WinError.h.
        error.kind() == std::io::ErrorKind::ConnectionReset
    } else {
        error.kind() == std::io::ErrorKind::ConnectionRefused
    }
}

/// Wrap a socket error.
fn network(error: &std::io::Error) -> RegistError {
    if is_would_block(error) {
        RegistError::Timeout
    } else {
        RegistError::Network(error.to_string())
    }
}

/// Wrap a setup error.
fn net(error: std::io::Error) -> RegistError {
    RegistError::Network(error.to_string())
}

/// Read the console's inner header block, the way the official client does.
fn parse_response_payload(payload: &[u8]) -> Result<PairedHost, RegistError> {
    let mut host = PairedHost {
        nickname: String::new(),
        mac: [0; 6],
        regist_key: [0; 0x10],
        rp_key: [0; 0x10],
        rp_key_type: 0,
        ap_ssid: String::new(),
        ap_bssid: String::new(),
        ap_key: String::new(),
        ap_name: String::new(),
        console_pin: 0,
    };
    let mut mac_found = false;
    let mut regist_found = false;
    let mut key_found = false;

    for (key, value) in headers_of(payload) {
        match key.as_str() {
            "AP-Ssid" => host.ap_ssid = value,
            "AP-Bssid" => host.ap_bssid = value,
            "AP-Key" => host.ap_key = value,
            "AP-Name" => host.ap_name = value,
            "PS5-Nickname" => host.nickname = value,
            "PS5-RegistKey" => match parse_hex::<0x10>(&value) {
                Some(key) => {
                    host.regist_key = key;
                    regist_found = true;
                }
                None => {
                    return Err(RegistError::BadResponse(
                        "the regist key was not valid hex".into(),
                    ))
                }
            },
            "RP-KeyType" => host.rp_key_type = value.trim().parse().unwrap_or(0),
            "RP-Key" => match parse_hex::<0x10>(&value) {
                Some(key) => {
                    host.rp_key = key;
                    key_found = true;
                }
                None => return Err(RegistError::BadResponse("the key was not valid hex".into())),
            },
            "PS5-Mac" => match parse_hex::<6>(&value) {
                Some(mac) => {
                    host.mac = mac;
                    mac_found = true;
                }
                None => return Err(RegistError::BadResponse("the MAC was not valid hex".into())),
            },
            _ => tracing::debug!("regist: ignoring {}", key),
        }
    }

    if !regist_found {
        return Err(RegistError::BadResponse("no regist key".into()));
    }
    if !key_found {
        return Err(RegistError::BadResponse("no RP key".into()));
    }
    if !mac_found {
        return Err(RegistError::BadResponse("no MAC address".into()));
    }
    Ok(host)
}

/// Split a byte slice of `Key: Value` lines into the pairs.
fn headers_of(text: &[u8]) -> Vec<(String, String)> {
    let text = String::from_utf8_lossy(text);
    text.split("\r\n")
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let (key, value) = line.split_once(':')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

/// Read a hex string of exactly `size` bytes into a fixed-size array.
fn parse_hex<const N: usize>(text: &str) -> Option<[u8; N]> {
    let text = text.trim();
    if text.len() != N * 2 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; N];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Lower-case hex, the way keys are written down.
pub fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Base64 of the 8-byte account ID, with padding, as the console expects.
fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(4 * bytes.len().div_ceil(3));
    for chunk in bytes.chunks(3) {
        let b0 = usize::from(chunk[0]);
        let b1 = chunk.get(1).map_or(0, |b| usize::from(*b));
        let b2 = chunk.get(2).map_or(0, |b| usize::from(*b));
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[(triple >> 18) & 0x3f] as char);
        out.push(TABLE[(triple >> 12) & 0x3f] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(triple >> 6) & 0x3f] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[triple & 0x3f] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake console: answers the search on UDP and the request on TCP, with an
    /// encrypted reply, so the whole exchange is tested without a PS5.
    struct FakeConsole {
        port: u16,
        thread: Option<std::thread::JoinHandle<()>>,
    }

    impl FakeConsole {
        /// Start a fake console that always answers `reply` (the decrypted inner header).
        fn start(reply: &str) -> FakeConsole {
            let udp = UdpSocket::bind("127.0.0.1:0").unwrap();
            let port = udp.local_addr().unwrap().port();
            let tcp = std::net::TcpListener::bind(("127.0.0.1", port)).unwrap();
            let reply = reply.as_bytes().to_vec();

            let thread = std::thread::spawn(move || {
                // The search packet first.
                let mut buf = [0u8; 64];
                let (n, from) = udp.recv_from(&mut buf).unwrap();
                assert_eq!(&buf[..n], SEARCH_PACKET);
                udp.send_to(SEARCH_REPLY, from).unwrap();

                let (mut stream, _) = tcp.accept().unwrap();
                let mut request = Vec::new();
                let mut chunk = [0u8; 2048];
                // Read the header and as many payload bytes as it promises.
                loop {
                    let read = stream.read(&mut chunk).unwrap();
                    request.extend_from_slice(&chunk[..read]);
                    if let Some(end) = find_subslice(&request, b"\r\n\r\n") {
                        let header = String::from_utf8_lossy(&request[..end + 4]).to_string();
                        let length = header.lines().find_map(|line| {
                            line.strip_prefix("Content-Length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        });
                        if let Some(length) = length {
                            if request.len() >= end + 4 + length {
                                break;
                            }
                        }
                    }
                }

                // Answer with the same cipher the client used: derive the
                // ambassador from the aeropause it sent, as a console does.
                let header_end = find_subslice(&request, b"\r\n\r\n").unwrap() + 4;
                let payload = &request[header_end..];
                let key_1_off = usize::from(payload[0] >> 3);
                // The client splits the aeropause across two offsets, as
                // chiaki's regist does: the first half at 0x191, the second at
                // 0xc7. Read both halves the way a console would.
                let mut aeropause = [0u8; 0x10];
                aeropause[..8].copy_from_slice(&payload[0x191..0x199]);
                aeropause[8..].copy_from_slice(&payload[0xc7..0xcf]);
                let mut ambassador = [0u8; 0x10];
                rpcrypt::ambassador_from_regist_aeropause(
                    Target::Ps5_1,
                    key_1_off,
                    &aeropause,
                    &mut ambassador,
                )
                .unwrap();
                let crypt = RPCrypt::init_regist(
                    Target::Ps5_1,
                    &ambassador,
                    usize::from(b'A' & 0x1F),
                    1234,
                )
                .unwrap();
                let mut body = reply.clone();
                let mut sealed = vec![0u8; body.len()];
                crypt.encrypt(0, &body, &mut sealed).unwrap();
                body.clear();
                body.extend_from_slice(&sealed);

                let answer = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len());
                stream.write_all(answer.as_bytes()).unwrap();
                stream.write_all(&body).unwrap();
            });

            FakeConsole {
                port,
                thread: Some(thread),
            }
        }
    }

    impl Drop for FakeConsole {
        fn drop(&mut self) {
            if let Some(thread) = self.thread.take() {
                thread.join().ok();
            }
        }
    }

    #[test]
    fn the_account_id_is_base64_with_padding() {
        assert_eq!(base64_encode(&[0u8; 8]), "AAAAAAAAAAA=");
        assert_eq!(base64_encode(&[0xffu8; 8]), "//////////8=");
        assert_eq!(base64_encode(&[1, 2, 3, 4, 5, 6, 7, 8]), "AQIDBAUGBwg=");
    }

    #[test]
    fn a_pasted_account_id_comes_back_as_8_bytes() {
        let pasted = base64_encode(&[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(
            decode_account_id(&pasted).unwrap(),
            [1, 2, 3, 4, 5, 6, 7, 8]
        );
        // What people actually paste varies, so hex and plain text work too.
        assert_eq!(
            decode_account_id("0102030405060708").unwrap(),
            [1, 2, 3, 4, 5, 6, 7, 8]
        );
        assert_eq!(decode_account_id("abcdefgh").unwrap(), *b"abcdefgh");
        assert!(decode_account_id("").is_err());
        assert!(decode_account_id("too short").is_err());
    }

    #[test]
    fn a_fake_console_pairs_and_hands_back_the_key() {
        let console = FakeConsole::start(
            "PS5-Nickname: Living room\r\n\
             PS5-RegistKey: 0123456789abcdef0123456789abcdef\r\n\
             RP-Key: 00112233445566778899aabbccddeeff\r\n\
             RP-KeyType: 0\r\n\
             PS5-Mac: 7cab12345678\r\n",
        );
        let steps = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = {
            let steps = steps.clone();
            let boxed: Box<dyn Fn(PairingStep) + Send> =
                Box::new(move |step: PairingStep| steps.lock().unwrap().push(step));
            boxed
        };
        let request = PairingRequest {
            address: "127.0.0.1",
            port: console.port,
            console: Console::Ps5,
            account_id: [0x11; 8],
            pin: 1234,
            on_step: Some(Box::new(move |step: PairingStep| sink(step))),
        };

        let host = pair(&request).expect("the fake console pairs");
        assert_eq!(host.nickname, "Living room");
        assert_eq!(
            host.regist_key,
            [
                0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
                0xcd, 0xef
            ]
        );
        assert_eq!(
            host.rp_key,
            [
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
                0xee, 0xff
            ]
        );
        assert_eq!(host.mac, [0x7c, 0xab, 0x12, 0x34, 0x56, 0x78]);
        assert_eq!(host.regist_key_hex(), "0123456789abcdef0123456789abcdef");
        assert_eq!(host.console_pin, 1234);

        let seen = steps.lock().unwrap().clone();
        assert!(seen.contains(&PairingStep::Searching));
        assert!(seen.contains(&PairingStep::Waiting));
        assert_eq!(*seen.last().unwrap(), PairingStep::Finishing);
    }

    #[test]
    fn a_console_with_no_key_tells_us_why() {
        let host =
            parse_response_payload(b"PS5-Nickname: Living room\r\nPS5-Mac: 7cab12345678\r\n");
        assert!(host.is_err(), "the regist key is missing");
    }

    #[test]
    fn a_console_with_a_short_key_tells_us_why() {
        let host = parse_response_payload(
            b"PS5-RegistKey: 0123\r\nRP-Key: 01\r\nPS5-Mac: 7cab12345678\r\n",
        );
        assert!(host.is_err());
    }

    #[test]
    fn a_console_that_never_answers_is_a_timeout_not_a_crash() {
        let udp = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = udp.local_addr().unwrap().port();
        let request = PairingRequest {
            address: "127.0.0.1",
            port,
            console: Console::Ps5,
            account_id: [0; 8],
            pin: 1234,
            on_step: None,
        };
        let error = pair(&request).expect_err("nobody answers");
        assert!(matches!(error, RegistError::Timeout), "got {error:?}");
    }

    #[test]
    fn hex_is_lower_case() {
        assert_eq!(hex(&[0xab, 0x0f]), "ab0f");
    }
}
