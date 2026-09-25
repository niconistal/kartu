# Handhelds

Kartu was designed around a real target: the **Miyoo Mini** (640 × 480, so 320 × 240 is an
exact ×2) running [Onion OS](https://onionui.github.io/). The same framebuffer player runs on
any Linux with `/dev/fb0` and evdev input; other handhelds (Anbernic, Trimui, …) are ports
waiting to happen.

## The framebuffer player

```sh
kartu play <cart> [--record FILE] [--audio DEV] [--mute] [--pick-out FILE]
```

- Draws through `/dev/fb0`. With two pages, `FBIOPAN_DISPLAY` is the flip; `FBIO_WAITFORVSYNC` on
  top would halve the frame rate, so it is only used single-buffered (`CW_VSYNC=1` forces it).
- Reads gamepads and keyboards through evdev.
- Sound goes to OSS `/dev/dsp` at 24 kHz mono (`--audio DEV`, `--mute`).
- `CW_ROTATE=0|180` overrides the panel orientation (the Miyoo's panel is mounted 180°).
- In game: **L2** shows a performance HUD, **R2** flips the picture, **MENU** exits.
- `--record FILE` saves your input as a replay script, as the web player's Record button does.

The Miyoo Mini runs it at a steady 60 fps: the console step is ~1.7 ms and the rest is the
framebuffer copy and rotate.

## Miyoo Mini / Onion OS

`scripts/build-miyoo.sh` builds a static ARMv7 binary (zig as the cross-linker) and assembles
`dist/miyoo/Kartu/`, an Onion **App** folder with the player, every cart and a menu.
`scripts/deliver-miyoo.sh` copies it to the device over ssh (`DEVICE_IP=…` or `[device.miyoo]`
in your config) and can run a benchmark when the device is idle.

```sh
scripts/build-miyoo.sh && DEVICE_IP=192.168.1.50 scripts/deliver-miyoo.sh
```

What lands on the SD card:

- **Apps → Kartu** (`/mnt/SDCARD/App/Kartu`): a menu of every cart (`miyoo/menu`, a Kartu cart
  itself; its list is generated from each cart's `-- title:` line). **A** plays, **MENU** in a
  game returns to the menu, **MENU** in the menu quits. The menu logs `PICK dir` and
  `kartu play --pick-out F` hands the choice to `launch.sh`.
- **Games → KARTU**: a Kartu console in the Games tab (`Emu/KARTU` from `miyoo/emu`) with one
  `Roms/KARTU/<Title>.kartu` pointer file per cart (its content is the cart folder), so each
  cart lists like a game with recents and favourites. MainUI needs a restart to list a new
  console.

### Sound on the Miyoo

`kartu` is static, so Onion's `libpadsp.so` preload can't route it through `audioserver`;
`launch.sh` runs Onion's `stop_audioserver.sh` (keeping the volume), the player writes 24 kHz
mono straight to `/dev/dsp` (the SigmaStar driver takes it as is), and Onion restarts
`audioserver` before MainUI. The driver returns its own error codes (`0xA005xxxx`) from
`write()`, hence raw `libc::write` and an error counter in the exit report;
`KARTU_OSS_DEBUG=N` logs the first N writes. `kartu.log` on the device shows the audio line
and dropped/error counts.

## Testing the framebuffer player without a device

```sh
modprobe vfb vfb_enable=1 videomemorysize=4000000
fbset -depth 32 -vyres 960
echo 0 > /sys/class/vtconsole/vtcon1/bind    # fbcon grabs it; unbind before rmmod vfb
kartu play carts/crypt
```

`qemu-arm` runs the ARM binary on an x86 box for a quick check that a build works.

## Porting to another handheld

Most Linux-based retro handhelds need three things: the right cross target (`armv7` or
`aarch64` musl), the framebuffer format (`fbdev.rs` handles 16- and 32-bit), and where the OS
expects apps to live (the Onion app folder is in `miyoo/`; other firmwares have their own).
Open an issue with the device name and we'll help — a port is a very welcome first contribution.
