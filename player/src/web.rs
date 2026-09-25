//! Web player: the same core compiled to WebAssembly. JS fetches the cart files, calls
//! `cw_boot`, then `cw_step` 60×/s and blits `cw_pixels()` (RGBA) to a canvas.

use kartu_core::{Console, H, W};
use std::cell::RefCell;

thread_local! {
    static CON: RefCell<Option<Console>> = const { RefCell::new(None) };
    static PIX: RefCell<Vec<u32>> = RefCell::new(vec![0; W * H]);
    static ERR: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static SND: RefCell<Vec<i16>> = RefCell::new(vec![0; kartu_core::audio::PER_FRAME]);
    // the cart's saved string: handed in by `cw_set_saved`, taken by the next `cw_boot`
    static SAVED: RefCell<Option<String>> = const { RefCell::new(None) };
    static SAVE_OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

unsafe fn text<'a>(p: *const u8, n: usize) -> &'a str {
    std::str::from_utf8(std::slice::from_raw_parts(p, n)).unwrap_or("")
}

fn set_err(e: &str) {
    ERR.with(|x| {
        let mut v = x.borrow_mut();
        v.clear();
        v.extend_from_slice(e.as_bytes());
        v.push(0);
    });
}

/// Returns 0 on success; on failure `cw_error()` says why.
#[no_mangle]
pub unsafe extern "C" fn cw_boot(a: *const u8, al: usize, l: *const u8, ll: usize, seed: u32) -> i32 {
    let saved = SAVED.with(|s| s.borrow().clone());
    match Console::new_saved(text(a, al), text(l, ll), seed as u64, saved) {
        Ok(c) => {
            CON.with(|x| *x.borrow_mut() = Some(c));
            set_err("");
            0
        }
        Err(e) => {
            set_err(&e);
            1
        }
    }
}

#[no_mangle]
pub extern "C" fn cw_step(buttons: u32) {
    CON.with(|x| {
        if let Some(c) = x.borrow_mut().as_mut() {
            c.step(buttons as u16);
            if let Some(e) = &c.error {
                set_err(e);
            }
        }
    });
}

/// RGBA8888 (canvas ImageData order) for the last rendered frame.
#[no_mangle]
pub extern "C" fn cw_pixels() -> *const u32 {
    CON.with(|x| {
        PIX.with(|p| {
            let mut p = p.borrow_mut();
            if let Some(c) = x.borrow().as_ref() {
                let s = c.st.borrow();
                let lut = s.gfx.lut(|[r, g, b]| u32::from_le_bytes([r, g, b, 255]));
                for (o, i) in p.iter_mut().zip(&s.gfx.fb) {
                    *o = lut[*i as usize];
                }
            }
            p.as_ptr()
        })
    })
}

#[no_mangle]
pub extern "C" fn cw_hash_hi() -> u32 {
    CON.with(|x| x.borrow().as_ref().map(|c| (c.hash() >> 32) as u32).unwrap_or(0))
}

#[no_mangle]
pub extern "C" fn cw_hash_lo() -> u32 {
    CON.with(|x| x.borrow().as_ref().map(|c| c.hash() as u32).unwrap_or(0))
}

#[no_mangle]
pub extern "C" fn cw_frame() -> u32 {
    CON.with(|x| x.borrow().as_ref().map(|c| c.frame() as u32).unwrap_or(0))
}

/// The cart's saved string (what its `load()` returns), from the page's storage. Call it
/// before `cw_boot` so `init()` sees it; called later it still reaches the running cart
/// before its next step. Length 0 = nothing saved.
#[no_mangle]
pub unsafe extern "C" fn cw_set_saved(p: *const u8, n: usize) {
    let s = (n > 0).then(|| text(p, n).to_string());
    SAVED.with(|x| *x.borrow_mut() = s.clone());
    CON.with(|x| {
        if let Some(c) = x.borrow_mut().as_mut() {
            c.set_saved(s);
        }
    });
}

/// NUL-terminated `save()` written since the last call, "" when there is none. The page
/// polls it after each step and stores what it gets.
#[no_mangle]
pub extern "C" fn cw_take_save() -> *const u8 {
    let s = CON.with(|x| x.borrow_mut().as_mut().and_then(|c| c.take_save()));
    SAVE_OUT.with(|x| {
        let mut v = x.borrow_mut();
        v.clear();
        v.extend_from_slice(s.as_deref().unwrap_or("").as_bytes());
        v.push(0);
        v.as_ptr()
    })
}

/// NUL-terminated last error, empty when fine.
#[no_mangle]
pub extern "C" fn cw_error() -> *const u8 {
    ERR.with(|x| {
        let mut v = x.borrow_mut();
        if v.is_empty() {
            v.push(0);
        }
        v.as_ptr()
    })
}

/// The last frame's sound: `cw_audio_len()` i16 samples, 24 kHz mono.
#[no_mangle]
pub extern "C" fn cw_audio() -> *const i16 {
    CON.with(|x| {
        SND.with(|s| {
            let mut s = s.borrow_mut();
            if let Some(c) = x.borrow().as_ref() {
                s.copy_from_slice(&c.st.borrow().audio.out);
            }
            s.as_ptr()
        })
    })
}

#[no_mangle]
pub extern "C" fn cw_audio_len() -> u32 {
    kartu_core::audio::PER_FRAME as u32
}
