//! Check the PC for the parts that make the stream fast.
//!
//! This is a read-only check. It looks for an NVIDIA GPU, FFmpeg, and the
//! NVDEC decoder. The video pipeline itself comes in a later milestone.

use serde::Serialize;
use std::process::Command;

/// Basic NVIDIA GPU details from `nvidia-smi`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GpuInfo {
    pub name: String,
    pub driver: String,
    pub memory: String,
}

/// What this PC can do today.
#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub gpu: Option<GpuInfo>,
    pub ffmpeg: Option<String>,
    pub nvdec: bool,
    pub os: String,
    pub arch: String,
}

/// Run the checks. Missing tools are reported as `None`, not as errors.
pub fn detect() -> SystemInfo {
    let gpu = nvidia_gpu();
    let ffmpeg =
        run("ffmpeg", &["-version"]).and_then(|text| text.lines().next().map(str::to_string));
    let nvdec = run("ffmpeg", &["-hide_banner", "-decoders"])
        .map(|text| text.contains("h264_cuvid") || text.contains("hevc_cuvid"))
        .unwrap_or(false);

    SystemInfo {
        gpu,
        ffmpeg,
        nvdec,
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    }
}

fn nvidia_gpu() -> Option<GpuInfo> {
    let output = run(
        "nvidia-smi",
        &[
            "--query-gpu=name,driver_version,memory.total",
            "--format=csv,noheader",
        ],
    )?;
    parse_gpu(output.lines().next()?)
}

fn parse_gpu(line: &str) -> Option<GpuInfo> {
    let mut parts = line.split(',').map(str::trim);
    let name = parts.next()?.to_string();
    if name.is_empty() {
        return None;
    }
    Some(GpuInfo {
        name,
        driver: parts.next().unwrap_or("unknown").to_string(),
        memory: parts.next().unwrap_or("unknown").to_string(),
    })
}

fn run(program: &str, args: &[&str]) -> Option<String> {
    let mut command = Command::new(program);
    command.args(args);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Do not flash a console window when the GUI asks for this.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_nvidia_smi_line() {
        let gpu = parse_gpu("NVIDIA GeForce RTX 4060 Ti, 621.40, 16380 MiB").unwrap();
        assert_eq!(gpu.name, "NVIDIA GeForce RTX 4060 Ti");
        assert_eq!(gpu.driver, "621.40");
        assert_eq!(gpu.memory, "16380 MiB");
    }

    #[test]
    fn rejects_empty_gpu_line() {
        assert!(parse_gpu("").is_none());
    }
}
