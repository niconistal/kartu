//! Linux framebuffer + evdev host: no SDL, no GPU, just /dev/fb0 and /dev/input.
//! This is the Miyoo Mini path (640×480, panel mounted upside down → rotate 180) and
//! works on any fbdev Linux handheld.
//!
//! Keys (Miyoo Mini keycodes): d-pad, A B X Y, L R, START SELECT as the console's
//! buttons; MENU quits; L2 toggles the perf HUD; R2 flips the picture 180°.
//! Sound goes to /dev/dsp (OSS, see oss.rs); `--audio DEV` picks another, `--mute` none.

use crate::cart;
use crate::headless::Stats;
use crate::Args;
use kartu_core::{Console, H, W};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::time::{Duration, Instant};

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Bitfield {
    offset: u32,
    length: u32,
    msb_right: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct VarInfo {
    xres: u32,
    yres: u32,
    xres_virtual: u32,
    yres_virtual: u32,
    xoffset: u32,
    yoffset: u32,
    bits_per_pixel: u32,
    grayscale: u32,
    red: Bitfield,
    green: Bitfield,
    blue: Bitfield,
    transp: Bitfield,
    nonstd: u32,
    activate: u32,
    height: u32,
    width: u32,
    accel_flags: u32,
    pixclock: u32,
    left_margin: u32,
    right_margin: u32,
    upper_margin: u32,
    lower_margin: u32,
    hsync_len: u32,
    vsync_len: u32,
    sync: u32,
    vmode: u32,
    rotate: u32,
    colorspace: u32,
    reserved: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FixInfo {
    id: [u8; 16],
    smem_start: libc::c_ulong,
    smem_len: u32,
    type_: u32,
    type_aux: u32,
    visual: u32,
    xpanstep: u16,
    ypanstep: u16,
    ywrapstep: u16,
    line_length: u32,
    mmio_start: libc::c_ulong,
    mmio_len: u32,
    accel: u32,
    capabilities: u16,
    reserved: [u16; 2],
}

const FBIOGET_VSCREENINFO: libc::c_ulong = 0x4600;
const FBIOPUT_VSCREENINFO: libc::c_ulong = 0x4601;
const FBIOGET_FSCREENINFO: libc::c_ulong = 0x4602;
const FBIOPAN_DISPLAY: libc::c_ulong = 0x4606;
const FBIO_WAITFORVSYNC: libc::c_ulong = 0x40044620;

struct Fb {
    file: File,
    var: VarInfo,
    line_len: usize,
    mem: *mut u8,
    mem_len: usize,
    pages: usize,
    page: usize,
    bpp: usize,
    scale: usize,
    rotate: bool,
    vsync: bool,
    line: Vec<u32>,
    pub info: String,
}

unsafe fn ioctl<T>(fd: i32, req: libc::c_ulong, arg: *mut T) -> i32 {
    libc::ioctl(fd, req as _, arg)
}

impl Fb {
    fn open(path: &str, rotate: bool) -> Result<Fb, String> {
        let file = OpenOptions::new().read(true).write(true).open(path).map_err(|e| format!("{path}: {e}"))?;
        let fd = file.as_raw_fd();
        let mut var = VarInfo::default();
        let mut fix: FixInfo = unsafe { std::mem::zeroed() };
        unsafe {
            if ioctl(fd, FBIOGET_VSCREENINFO, &mut var) != 0 || ioctl(fd, FBIOGET_FSCREENINFO, &mut fix) != 0 {
                return Err("framebuffer ioctls failed".into());
            }
        }
        if var.yres_virtual < var.yres * 2 {
            let mut want = var;
            want.yres_virtual = var.yres * 2;
            unsafe {
                if ioctl(fd, FBIOPUT_VSCREENINFO, &mut want) == 0 {
                    ioctl(fd, FBIOGET_VSCREENINFO, &mut var);
                    ioctl(fd, FBIOGET_FSCREENINFO, &mut fix);
                }
            }
        }
        let bpp = var.bits_per_pixel as usize;
        if bpp != 32 && bpp != 16 {
            return Err(format!("unsupported framebuffer depth {bpp}"));
        }
        let line_len = fix.line_length as usize;
        let page_bytes = line_len * var.yres as usize;
        let mem_len = fix.smem_len as usize;
        let pages = if var.yres_virtual >= var.yres * 2 && mem_len >= page_bytes * 2 { 2 } else { 1 };
        let mem = unsafe { libc::mmap(std::ptr::null_mut(), mem_len, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_SHARED, fd, 0) };
        if mem == libc::MAP_FAILED {
            return Err("mmap of framebuffer failed".into());
        }
        let scale = ((var.xres as usize / W).min(var.yres as usize / H)).max(1);
        let mut dummy: u32 = 0;
        // With two pages, PAN_DISPLAY is the flip (and on many drivers already waits for
        // vblank); waiting again on top would halve the frame rate. CW_VSYNC=1 forces it.
        let want_vsync = match std::env::var("CW_VSYNC").ok().as_deref() {
            Some("1") => true,
            Some(_) => false,
            None => pages == 1,
        };
        let vsync = want_vsync && unsafe { ioctl(fd, FBIO_WAITFORVSYNC, &mut dummy) } == 0;
        unsafe { std::ptr::write_bytes(mem as *mut u8, 0, mem_len.min(page_bytes * pages)) };
        let info = format!(
            "fb {}x{} virt {}x{} {}bpp line {} pages {} scale x{} rotate {} vsync-ioctl {} rgb offs {}/{}/{}",
            var.xres, var.yres, var.xres_virtual, var.yres_virtual, bpp, line_len, pages, scale, if rotate { 180 } else { 0 }, vsync,
            var.red.offset, var.green.offset, var.blue.offset
        );
        Ok(Fb { file, var, line_len, mem: mem as *mut u8, mem_len, pages, page: 0, bpp, scale, rotate, vsync, line: vec![0; W * scale], info })
    }

    fn pack(&self) -> impl Fn([u8; 3]) -> u32 + '_ {
        move |[r, g, b]: [u8; 3]| {
            let v = &self.var;
            let ch = |c: u8, bf: &Bitfield| ((c as u32) >> (8 - bf.length.min(8))) << bf.offset;
            ch(r, &v.red) | ch(g, &v.green) | ch(b, &v.blue) | if v.transp.length > 0 { ((1u32 << v.transp.length) - 1) << v.transp.offset } else { 0 }
        }
    }

    fn present(&mut self, c: &Console) {
        let s = c.st.borrow();
        let lut = s.gfx.lut(self.pack());
        let sc = self.scale;
        let (xres, yres) = (self.var.xres as usize, self.var.yres as usize);
        let ow = W * sc;
        let ox = (xres - ow) / 2;
        let oy = (yres - H * sc) / 2;
        let page_off = if self.pages == 2 { self.page * yres * self.line_len } else { 0 };
        let bytes = self.bpp / 8;
        for y in 0..H {
            let src = &s.gfx.fb[y * W..(y + 1) * W];
            if self.rotate {
                for (x, &i) in src.iter().rev().enumerate() {
                    self.line[x * sc..x * sc + sc].fill(lut[i as usize]);
                }
            } else {
                for (x, &i) in src.iter().enumerate() {
                    self.line[x * sc..x * sc + sc].fill(lut[i as usize]);
                }
            }
            for k in 0..sc {
                let mut dy = oy + y * sc + k;
                let mut dx = ox;
                if self.rotate {
                    dy = yres - 1 - dy;
                    dx = xres - ox - ow;
                }
                let off = page_off + dy * self.line_len + dx * bytes;
                unsafe {
                    let dst = self.mem.add(off);
                    if bytes == 4 {
                        std::ptr::copy_nonoverlapping(self.line.as_ptr() as *const u8, dst, ow * 4);
                    } else {
                        let d = dst as *mut u16;
                        for (x, p) in self.line.iter().enumerate() {
                            *d.add(x) = *p as u16;
                        }
                    }
                }
            }
        }
        drop(s);
        let fd = self.file.as_raw_fd();
        if self.vsync {
            let mut dummy: u32 = 0;
            unsafe { ioctl(fd, FBIO_WAITFORVSYNC, &mut dummy) };
        }
        if self.pages == 2 {
            self.var.yoffset = (self.page * yres) as u32;
            unsafe { ioctl(fd, FBIOPAN_DISPLAY, &mut self.var) };
            self.page ^= 1;
        }
    }
}

impl Drop for Fb {
    fn drop(&mut self) {
        unsafe {
            std::ptr::write_bytes(self.mem, 0, self.mem_len.min(self.line_len * self.var.yres as usize * self.pages));
            if self.pages == 2 {
                self.var.yoffset = 0;
                ioctl(self.file.as_raw_fd(), FBIOPAN_DISPLAY, &mut self.var);
            }
            libc::munmap(self.mem as *mut libc::c_void, self.mem_len);
        }
    }
}

/// Miyoo Mini keycodes → console button bit (BUTTONS order in core).
fn key_to_bit(code: u16) -> Option<u16> {
    let i = match code {
        105 => 0,          // left
        106 => 1,          // right
        103 => 2,          // up
        108 => 3,          // down
        57 => 4,           // A (space)
        29 => 5,           // B (left ctrl)
        42 => 6,           // X (left shift)
        56 => 7,           // Y (left alt)
        18 => 8,           // L1 (e)
        20 => 9,           // R1 (t)
        28 => 10,          // START (enter)
        97 => 11,          // SELECT (right ctrl)
        _ => return None,
    };
    Some(1 << i)
}

const KEY_MENU: u16 = 1;
const KEY_L2: u16 = 15;
const KEY_R2: u16 = 14;

struct Input {
    devs: Vec<File>,
    pub bits: u16,
    pub quit: bool,
    pub toggles: Vec<u16>,
}

impl Input {
    fn open() -> Input {
        let mut devs = vec![];
        if let Ok(rd) = std::fs::read_dir("/dev/input") {
            for e in rd.flatten() {
                if e.file_name().to_string_lossy().starts_with("event") {
                    if let Ok(f) = OpenOptions::new().read(true).custom_flags(libc::O_NONBLOCK).open(e.path()) {
                        devs.push(f);
                    }
                }
            }
        }
        Input { devs, bits: 0, quit: false, toggles: vec![] }
    }

    fn poll(&mut self) {
        // struct input_event: timeval (2 × long) + u16 type + u16 code + i32 value
        let tv = 2 * std::mem::size_of::<libc::c_ulong>();
        let sz = tv + 8;
        let mut buf = [0u8; 64 * 24];
        for d in &mut self.devs {
            while let Ok(n) = d.read(&mut buf[..sz * 64]) {
                if n == 0 {
                    break;
                }
                for ev in buf[..n].chunks_exact(sz) {
                    let ty = u16::from_ne_bytes([ev[tv], ev[tv + 1]]);
                    let code = u16::from_ne_bytes([ev[tv + 2], ev[tv + 3]]);
                    let val = i32::from_ne_bytes([ev[tv + 4], ev[tv + 5], ev[tv + 6], ev[tv + 7]]);
                    if ty != 1 {
                        continue; // EV_KEY only
                    }
                    if val == 1 && matches!(code, KEY_MENU | KEY_L2 | KEY_R2) {
                        if code == KEY_MENU {
                            self.quit = true;
                        }
                        self.toggles.push(code);
                    }
                    if let Some(b) = key_to_bit(code) {
                        if val == 0 {
                            self.bits &= !b;
                        } else {
                            self.bits |= b;
                        }
                    }
                }
            }
        }
    }
}

extern "C" fn on_signal(_: libc::c_int) {
    STOP.store(true, std::sync::atomic::Ordering::SeqCst);
}
static STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn play(a: &[String]) -> Result<(), String> {
    let a = Args::parse(a);
    let cart_dir = a.cart()?;
    let rotate = match std::env::var("CW_ROTATE").ok().as_deref() {
        Some("180") => true,
        Some(_) => false,
        None => std::path::Path::new("/mnt/SDCARD").exists(),
    };
    let mut bench_secs: f64 = a.num("bench-secs", 0.0);
    let then_play = a.kv.contains_key("then-play");
    let mut bench_written = false;
    let out_path = a.kv.get("out").cloned();
    let mut fb = Fb::open(a.kv.get("fb").map(|s| s.as_str()).unwrap_or("/dev/fb0"), rotate)?;
    let mut input = Input::open();
    unsafe {
        libc::signal(libc::SIGINT, on_signal as extern "C" fn(libc::c_int) as usize);
        libc::signal(libc::SIGTERM, on_signal as extern "C" fn(libc::c_int) as usize);
    }
    // The cart's save()/load() string is <cart>/save.txt; a save is written the frame it's made.
    let save_path = std::path::Path::new(&cart_dir).join("save.txt");
    let saved = std::fs::read_to_string(&save_path).ok();
    let mut save_err = false;
    let mut c = cart::boot(&cart_dir, a.num("seed", 1), saved)?;
    // Sound: /dev/dsp unless --mute. No device (or it's busy) = play silently, say why.
    let dsp = a.kv.get("audio").cloned().unwrap_or_else(|| "/dev/dsp".into());
    let (mut oss, audio_info) = if a.kv.contains_key("mute") {
        (None, "audio off (--mute)".to_string())
    } else {
        match crate::oss::Oss::open(&dsp) {
            Ok(o) => {
                let i = o.info.clone();
                (Some(o), i)
            }
            Err(e) => (None, format!("audio off: {e}")),
        }
    };
    let mut rec = a.kv.get("record").map(|_| cart::Recorder::default());
    let mut report = vec![
        format!("kartu {} {} · cart {cart_dir}", env!("CARGO_PKG_VERSION"), std::env::consts::ARCH),
        fb.info.clone(),
        format!("input devices: {}", input.devs.len()),
        audio_info,
    ];
    println!("{}", report.join("\n"));

    let frame = Duration::from_nanos(16_666_667);
    let start = Instant::now();
    let mut next = start + frame;
    let mut hud = true;
    let (mut st, mut pr, mut iv) = (Stats::new(), Stats::new(), Stats::new());
    let (mut win_step, mut win_pres, mut win_iv) = (Stats::new(), Stats::new(), Stats::new());
    let mut misses = 0u32;
    let mut win_misses = 0u32;
    let mut last = Instant::now();
    let mut last_log = Instant::now();
    let mut phase = 0; // bench: 0 gate, 1 stress
    let mut hud_text = String::new();

    loop {
        input.poll();
        for t in input.toggles.drain(..) {
            match t {
                KEY_L2 => hud = !hud,
                KEY_R2 => fb.rotate = !fb.rotate,
                _ => {}
            }
        }
        if input.quit || STOP.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        let elapsed = start.elapsed().as_secs_f64();
        let mut bits = input.bits;
        if bench_secs > 0.0 {
            if phase == 0 && elapsed > bench_secs / 2.0 {
                report.push(format!("GATE   (64 spr, 2 layers)   step {} | present {} | interval {} | misses {}", st.summary(), pr.summary(), iv.summary(), misses));
                (st, pr, iv, misses) = (Stats::new(), Stats::new(), Stats::new(), 0);
                bits |= 1 << 4; // A → stress mode
                phase = 1;
            } else if elapsed > bench_secs {
                report.push(format!("STRESS (128 spr, 4 layers) step {} | present {} | interval {} | misses {}", st.summary(), pr.summary(), iv.summary(), misses));
                if !then_play {
                    break;
                }
                // Hand the scene back to the player: write the numbers now, keep running.
                report.push(format!("bench done at frame {} · hash {:016x}", c.frame(), c.hash()));
                write_report(&report, out_path.as_deref());
                (st, pr, iv, misses) = (Stats::new(), Stats::new(), Stats::new(), 0);
                bits |= 1 << 4; // back to the gate scene
                bench_secs = 0.0;
                bench_written = true;
            }
        }

        if let Some(r) = rec.as_mut() {
            r.push(c.frame(), bits);
        }
        let t0 = Instant::now();
        c.step(bits);
        if let Some(o) = oss.as_mut() {
            o.push(&c.st.borrow().audio.out);
        }
        let t1 = Instant::now();
        if let Some(s) = c.take_save() {
            if let Err(e) = write_save(&save_path, &s) {
                if !save_err {
                    eprintln!("save.txt: {e} (the cart's saves are lost this session)");
                    save_err = true;
                }
            }
        }
        let mut picked = false;
        for l in c.take_logs() {
            println!("[cart] {l}");
            // The Miyoo menu cart says `PICK <cart>`; launch.sh reads the file and plays it.
            if let (Some(name), Some(p)) = (l.strip_prefix("PICK "), a.kv.get("pick-out")) {
                std::fs::write(p, name.trim()).map_err(|e| format!("{p}: {e}"))?;
                picked = true;
            }
        }
        if picked {
            break;
        }
        if hud {
            let mut s = c.st.borrow_mut();
            s.gfx.cram[510] = [0x10, 0x10, 0x18];
            s.gfx.cram[511] = [0xa7, 0xf0, 0x70];
            let w = hud_text.len() as i32 * 8 + 4;
            s.gfx.fill_rect(W as i32 - w, H as i32 - 11, w, 11, 510);
            s.gfx.text(&hud_text, W as i32 - w + 2, H as i32 - 9, 511);
        }
        fb.present(&c);
        let t2 = Instant::now();

        let step_ms = (t1 - t0).as_secs_f64() * 1e3;
        let pres_ms = (t2 - t1).as_secs_f64() * 1e3;
        let interval = (t2 - last).as_secs_f64() * 1e3;
        last = t2;
        st.add(step_ms);
        pr.add(pres_ms);
        win_step.add(step_ms);
        win_pres.add(pres_ms);
        if c.frame() > 2 {
            iv.add(interval);
            win_iv.add(interval);
            if interval > 16.667 * 1.5 {
                misses += 1;
                win_misses += 1;
            }
        }

        if last_log.elapsed() >= Duration::from_secs(1) {
            let fps = 1000.0 / win_iv.avg().max(0.001);
            hud_text = format!("{fps:.1}fps cpu {:.1}+{:.1}ms", win_step.avg(), win_pres.avg());
            if (start.elapsed().as_secs() % 5) == 0 {
                println!("t={:.0}s fps {fps:.1} step {:.2}ms present {:.2}ms misses {win_misses}", start.elapsed().as_secs_f64(), win_step.avg(), win_pres.avg());
            }
            win_step.clear();
            win_pres.clear();
            win_iv.clear();
            win_misses = 0;
            last_log = Instant::now();
        }

        // Fixed timestep pacing. If vsync already blocked us, we're on time and this is a no-op.
        let now = Instant::now();
        if now < next {
            std::thread::sleep(next - now);
            next += frame;
        } else if now - next > frame * 3 {
            next = now + frame; // fell far behind (debugger, suspend): don't sprint to catch up
        } else {
            next += frame;
        }
    }
    if let Some(e) = &c.error {
        report.push(format!("cart error: {e}"));
    }
    if (bench_secs <= 0.0 || bench_written) && st.len() > 0 {
        report.push(format!("session step {} | present {} | interval {} | misses {}", st.summary(), pr.summary(), iv.summary(), misses));
    }
    report.push(format!("frames {} · final hash {:016x}", c.frame(), c.hash()));
    if let Some(o) = &oss {
        report.push(format!("audio: {} samples dropped (device slower than 60 Hz), {} driver errors", o.dropped, o.errors));
    }
    if bench_written {
        println!("{}", report.join("\n"));
    } else {
        write_report(&report, out_path.as_deref());
    }
    if let (Some(r), Some(p)) = (rec, a.kv.get("record")) {
        std::fs::write(p, &r.out).map_err(|e| format!("{p}: {e}"))?;
        println!("input recorded to {p}");
    }
    Ok(())
}

/// Write through a temp file and rename, so a power cut mid-write can't leave half a save.
fn write_save(path: &std::path::Path, s: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("txt.tmp");
    std::fs::write(&tmp, s)?;
    std::fs::rename(&tmp, path)
}

fn write_report(report: &[String], out: Option<&str>) {
    let text = report.join("\n");
    println!("{text}");
    if let Some(p) = out {
        if let Ok(mut f) = File::create(p) {
            writeln!(f, "{text}").ok();
        }
    }
}
