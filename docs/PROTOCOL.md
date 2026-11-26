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

## Next: pairing

1. Ask the console for a PIN.
2. Show the PIN on the PC.
3. Send the PIN back with the PSN account ID.
4. Save the console's pairing key and target type.

This is the next milestone.

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
