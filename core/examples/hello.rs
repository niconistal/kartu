//! Run a tiny cart headless for two seconds and save the last frame as a PPM image.
//!
//!     cargo run -p kartu-core --example hello -- hello.ppm

use kartu_core::{Console, BUTTONS, H, W};

const ASSETS: &str = "\
palette p
  . clear
  k #1a1c2c
  w #f4f4f4
  y #ffcd75
sprite star 8x8 pal=p
...yy...
...yy...
yyyyyyyy
.yyyyyy.
..yyyy..
.yy..yy.
yy....yy
........
";

const MAIN: &str = r##"
x, y = 156, 116
function update()
  if btn("left") then x = x - 2 end
  if btn("right") then x = x + 2 end
end
function draw()
  backdrop("#1a1c2c")
  text("HELLO, KARTU", 112, 40, "w")
  spr("star", x, y + sin(frame() / 60) * 8)
end
"##;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "hello.ppm".into());
    let mut c = Console::new(ASSETS, MAIN, 1).expect("cart failed to load");
    let right = 1 << BUTTONS.iter().position(|b| *b == "right").unwrap();
    for i in 0..120 {
        c.step(if i < 30 { right } else { 0 });
    }
    if let Some(e) = &c.error {
        eprintln!("cart error: {e}");
        std::process::exit(1);
    }

    let s = c.st.borrow();
    let lut = s.gfx.lut(|[r, g, b]| (r as u32) << 16 | (g as u32) << 8 | b as u32);
    let mut ppm = format!("P6\n{W} {H}\n255\n").into_bytes();
    for &i in &s.gfx.fb {
        let rgb = lut[i as usize];
        ppm.extend([(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8]);
    }
    std::fs::write(&out, ppm).expect("write image");
    println!("frame {} hash {:016x} -> {out}", c.frame(), c.hash());
}
