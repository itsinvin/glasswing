# Protocol notes

This page records what Glasswing implements and what it will implement. It is
a working note, not a complete protocol spec.

Protocol details are based on [chiaki-ng](https://github.com/chiaki-ng/chiaki-ng)
and [KyberNow](https://github.com/MonsieurNours/kybernow), both AGPL-3.0.

## Discovery (done)

- Transport: UDP, IPv4.
- Console port: **9302**.
- Local port: first free port in **9303–9319**, otherwise any free port.
- Request packet (a trailing zero byte is included):

```
SRCH * HTTP/1.1
device-discovery-protocol-version:00030010
\0
```

- Reply: an HTTP-like response. Status **200** means ready; **620** means rest
  mode.
- Useful headers: `host-id`, `host-name`, `host-type`, `host-request-port`,
  `system-version`, `device-discovery-protocol-version`,
  `running-app-titleid`, `running-app-name`.

The scanner sends to `255.255.255.255`, to each interface's broadcast address,
and to any saved console addresses. It repeats every 500 ms while scanning.

## Wake (packet done, needs pairing key)

- Transport: UDP, port **9302**.
- Packet:

```
WAKEUP * HTTP/1.1
client-type:vr
auth-type:R
model:w
app-type:r
user-credential:<regist key as a number>
device-discovery-protocol-version:00030010
\0
```

The `user-credential` is the console's pairing key read as a hex number. A
console will ignore a wake packet without the right key. That is why wake is
not useful until pairing exists.

## Pairing ("regist", done)

Pairing is its own little server on the console, separate from discovery.

- **Step 1, search.** UDP to **9295**. The client sends `SRC3\0` and waits for
  a reply starting `RES3`. The console only answers while its
  *Settings > Linked Devices > Link Device* screen is open, so a silent console
  almost always means that screen is closed, not that the console is gone.
- **Step 2, settle.** Wait 100 ms, then open a TCP connection to the same
  address on 9295.
- **Step 3, the request.** An HTTP POST to `/sie/ps5/rp/sess/rgst`, with
  `RP-Version: 1.0`, and a body the official client calls a payload. The
  console reads the body as headers.
- **Step 4, the exchange.** The console answers `HTTP 200` with its own
  headers, and the pairing is done. Both bodies are encrypted.

The cipher ("RP crypt") is in `crates/glasswing-core/src/rpcrypt.rs` and works
like this:

- The client invents a 16-byte **ambassador**, and derives a 16-byte **bright**
  from it, the PIN, and a byte of the payload's own padding. The bright is the
  AES key and is never sent.
- What travels is the **aeropause**: the ambassador scrambled with a table
  entry both sides can find. The console reads the ambassador back out of it,
  and so can this client, which is how the fake console in the tests can answer.
- The inner headers (`Client-Type`, `Np-AccountId`) are encrypted with
  AES-128-CFB128, no padding, keyed by the bright, with an IV that is
  HMAC-SHA256 of the ambassador and a counter.
- The console's reply uses the same key in the other direction.

The account ID is the PSN account ID from Settings, base64 of 8 bytes. In v1
the user pastes it; there is no login flow.

What the reply carries:

| Header | Meaning |
|---|---|
| `PS5-Nickname` | The name the owner gave the console |
| `PS5-RegistKey` | The 16-byte key. Sessions and wake packets use it |
| `RP-Key` | A second 16-byte key |
| `RP-KeyType` | Which kind of key `RP-Key` is |
| `PS5-Mac` | The console's MAC address |

If any of `PS5-RegistKey`, `RP-Key` or `PS5-Mac` is missing or not valid hex,
the reply is rejected, which catches a wrong PIN (the console answers, but the
headers came back unreadable).

Testing: `gw pair --address 192.168.1.24 --pin 12345678` runs the whole
exchange and saves the key. The in-memory fake console in `regist.rs` tests
does the same without a PlayStation on the network.

## Next: session and control channel

After pairing:

- HTTP request to the console's control port (from `host-request-port`).
- Open the control channel and keep it alive.
- Send the login PIN and wait for the session ID.
- Switch the session to stream mode.

## Next: stream

- Senkusha MTU and RTT probing.
- Takion BIG/BANG handshake.
- STREAMINFO.
- Takion AV packets: decrypt, reorder, FEC, assemble access units.
- Video: H.264 SDR, 1080p60.
- Audio: Opus, 48 kHz stereo.
- Feedback: controller state at 250 Hz.
- DualSense: controller type 6, trigger effects on message type 11, haptics
  as 3 kHz S16 stereo PCM in audio packets marked with `0x02`.

## Safety

This project is for playing your own games on your own console over your own
network. It does not bypass copy protection and does not include Sony code or
credentials.
