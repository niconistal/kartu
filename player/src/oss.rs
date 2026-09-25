//! OSS (`/dev/dsp`) sound output for the fbdev player: no ALSA library, just ioctls + write.
//! The Miyoo Mini's kernel exposes one; Onion's `audioserver` normally holds it, so launch.sh
//! stops that first (Onion's own `stop_audioserver.sh`, which keeps the volume) and restarts it after.
//!
//! The synth makes `audio::PER_FRAME` samples of 24 kHz mono per frame. If the device wants
//! another rate or stereo we resample linearly / duplicate. Writes never block the frame loop:
//! what doesn't fit waits in `pending`, and `pending` is capped so a device clock that runs a
//! little slower than our 60 Hz can't build up delay (we drop the oldest samples instead).

use kartu_core::audio::RATE;
use std::fs::{File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;

// _IOWR('P', n, int) / _IOR('P', 12, audio_buf_info) on Linux
const SNDCTL_DSP_SPEED: u32 = 0xC004_5002;
const SNDCTL_DSP_SETFMT: u32 = 0xC004_5005;
const SNDCTL_DSP_CHANNELS: u32 = 0xC004_5006;
const SNDCTL_DSP_SETFRAGMENT: u32 = 0xC004_500A;
const SNDCTL_DSP_GETOSPACE: u32 = 0x8010_500C;
const AFMT_S16_LE: i32 = 0x10;

#[repr(C)]
#[derive(Default)]
struct BufInfo {
    fragments: i32,
    fragstotal: i32,
    fragsize: i32,
    bytes: i32,
}

pub struct Oss {
    f: File,
    rate: u32,
    channels: u32,
    /// resampler position between the last two input samples, in input samples
    phase: f64,
    last: i16,
    /// interleaved device samples waiting for room in the driver
    pending: Vec<i16>,
    max_pending: usize,
    pub info: String,
    pub dropped: u64,
    /// writes the driver refused with an error code (not just "full")
    pub errors: u64,
    debug: u32,
}

fn ioctl_int(fd: i32, req: u32, v: i32) -> Result<i32, std::io::Error> {
    let mut x = v;
    if unsafe { libc::ioctl(fd, req as _, &mut x) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(x)
}

impl Oss {
    pub fn open(path: &str) -> Result<Oss, String> {
        let f = OpenOptions::new().write(true).custom_flags(libc::O_NONBLOCK).open(path).map_err(|e| format!("{path}: {e}"))?;
        let fd = f.as_raw_fd();
        // 8 fragments of 1 KiB: ~40 ms of 24 kHz stereo in the driver. A hint; drivers may ignore it.
        let _ = ioctl_int(fd, SNDCTL_DSP_SETFRAGMENT, (8 << 16) | 10);
        let fmt = ioctl_int(fd, SNDCTL_DSP_SETFMT, AFMT_S16_LE).map_err(|e| format!("{path}: SETFMT: {e}"))?;
        if fmt != AFMT_S16_LE {
            return Err(format!("{path}: no signed 16-bit format (got {fmt:#x})"));
        }
        let channels = ioctl_int(fd, SNDCTL_DSP_CHANNELS, 1).map_err(|e| format!("{path}: CHANNELS: {e}"))?.clamp(1, 2) as u32;
        let rate = ioctl_int(fd, SNDCTL_DSP_SPEED, RATE as i32).map_err(|e| format!("{path}: SPEED: {e}"))?;
        if !(8000..=192_000).contains(&rate) {
            return Err(format!("{path}: odd rate {rate}"));
        }
        let rate = rate as u32;
        let mut bi = BufInfo::default();
        let space = if unsafe { libc::ioctl(fd, SNDCTL_DSP_GETOSPACE as _, &mut bi) } == 0 {
            format!("{} × {} B", bi.fragstotal, bi.fragsize)
        } else {
            "?".into()
        };
        // ~4 frames of backlog at most, on top of what the driver buffers
        let max_pending = (rate as usize * channels as usize) / 15;
        let info = format!("audio {path} {rate} Hz {channels} ch s16 · driver buffer {space}");
        // Start with ~50 ms of silence queued: a cushion so one slow frame doesn't underrun (click).
        let pending = vec![0i16; (rate as usize / 20) * channels as usize];
        Ok(Oss { f, rate, channels, phase: 0.0, last: 0, pending, max_pending, info, dropped: 0, errors: 0, debug: std::env::var("KARTU_OSS_DEBUG").map(|v| v.parse().unwrap_or(40)).unwrap_or(0) })
    }

    /// One frame of 24 kHz mono from the synth.
    pub fn push(&mut self, src: &[i16]) {
        if self.rate == RATE {
            for &s in src {
                for _ in 0..self.channels {
                    self.pending.push(s);
                }
            }
        } else {
            // linear interpolation between `last` and each new input sample
            let step = RATE as f64 / self.rate as f64;
            let mut prev = self.last;
            for &s in src {
                while self.phase < 1.0 {
                    let v = prev as f64 + (s as f64 - prev as f64) * self.phase;
                    for _ in 0..self.channels {
                        self.pending.push(v as i16);
                    }
                    self.phase += step;
                }
                self.phase -= 1.0;
                prev = s;
            }
            self.last = prev;
        }
        if self.pending.len() > self.max_pending {
            let cut = self.pending.len() - self.max_pending;
            let cut = cut - cut % self.channels as usize;
            self.pending.drain(..cut);
            self.dropped += cut as u64;
        }
        self.flush();
    }

    fn flush(&mut self) {
        while !self.pending.is_empty() {
            let bytes = unsafe { std::slice::from_raw_parts(self.pending.as_ptr() as *const u8, self.pending.len() * 2) };
            // Raw write(2): the SigmaStar driver returns its own error codes (0xA005xxxx) as the
            // result, which std would take for a huge byte count. Anything < 0 is an error.
            let r = unsafe { libc::write(self.f.as_raw_fd(), bytes.as_ptr() as *const libc::c_void, bytes.len()) };
            if self.debug > 0 {
                self.debug -= 1;
                let mut bi = BufInfo::default();
                unsafe { libc::ioctl(self.f.as_raw_fd(), SNDCTL_DSP_GETOSPACE as _, &mut bi) };
                eprintln!("oss write {} B -> {r:#x} · space {} B", bytes.len(), bi.bytes);
            }
            if r <= 0 || r as usize > bytes.len() {
                if r < -1 || r as usize > bytes.len() {
                    self.errors += 1; // driver error code, not EAGAIN
                }
                break; // EAGAIN (driver full) or an error: try again next frame
            }
            // keep whole samples; a split one would swap byte order from here on
            let n = r as usize / 2 / self.channels as usize * self.channels as usize;
            self.pending.drain(..n);
            if n == 0 {
                break;
            }
        }
    }
}
