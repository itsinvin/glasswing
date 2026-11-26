# Performance notes

The goal is simple: make the time from "the PS5 draws a frame" to "you see it"
as short as possible. This page explains the plan and the PC settings that
help.

## What the app will do

1. **Decode on the GPU with NVDEC.** The stream is H.264. NVDEC decodes it on
   the NVIDIA GPU instead of the CPU. The first path uses FFmpeg's
   `h264_cuvid`; D3D11VA is the fallback.
2. **Draw with wgpu on DX12.** wgpu picks the high-performance adapter. The
   decoded frame is copied to a GPU texture with as little copying as
   possible.
3. **Show the newest frame.** If two frames are ready, the old one is dropped.
   A long queue makes the picture smooth but adds delay.
4. **Keep the network thread separate.** UDP receive, decryption, reordering,
   FEC, and frame assembly happen on one thread. Decode, render, audio, and
   input each have their own thread.
5. **No async in the hot path.** Async is fine for setup and menus. It is not
   used for packets or frames.
6. **Small audio buffer.** Start with 3 Opus frames, allow 8, drop the oldest
   if the buffer runs over.

## NVIDIA Control Panel

Set these for `glasswing.exe` (or globally while testing):

| Setting | Value | Why |
|---|---|---|
| Low Latency Mode | Ultra | Keeps the driver's queue short |
| Power management mode | Prefer maximum performance | Stops the GPU from downclocking |
| Vertical sync | Off | The app controls presentation |
| Texture filtering quality | Quality | No need to trade quality here |
| Shader cache size | Driver default | Fine for this app |

After a driver update, check these again. NVIDIA sometimes resets profiles.

## Windows settings

- **Hardware-accelerated GPU scheduling:** On (Settings → System → Display →
  Graphics → Change default graphics settings).
- **Game Mode:** On.
- **Fullscreen optimizations:** If you see extra delay or stutter, open the
  `glasswing.exe` properties → Compatibility → disable fullscreen
  optimizations.
- **Wired Ethernet:** Wi-Fi adds jitter. Use a cable for the best result.
- **Power plan:** High performance or Balanced with the PC plugged in.

## How we will measure

Guesses are not good enough. We will measure:

- **PresentMon** for frame times and display delay on the PC.
- **A 240 fps phone camera** filming a high-speed timer on the PC and the
  console at the same time. The gap between the two screens is the real
  glass-to-glass delay.
- **A packet log** for network time: receive, decrypt, reorder, and assemble.

The first target is **35–50 ms wired** with software decode. NVDEC should
bring that down. We will publish real numbers, not marketing numbers.

## Current status

The video path is **not built yet**. The app can check for an NVIDIA GPU,
FFmpeg, and NVDEC so you know the PC is ready. The settings are saved for the
first stream milestone.

The window itself is native Rust on Rune, which presents with DirectX 12 and
paces frames off the display, so a game frame can be drawn into a Rune surface
and shown without extra copies or a second compositor.

## What not to do

- Do not add a big frame queue to hide stutter. It adds delay.
- Do not decode on the UI thread.
- Do not use a timer to pace frames when the display can pace them.
- Do not copy full frames more than needed. Use GPU textures.
- Do not trust "it feels fast." Measure it.
