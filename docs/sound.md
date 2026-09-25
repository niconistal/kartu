# Sound

Kartu has an 8-voice synthesiser inside the console: 24 kHz mono, 400 samples per frame, the
same samples on every target. Songs use up to 6 voices; sound effects get the rest. Everything
is written as text in `assets.cw`, and **everything works with no setup** — the console ships
with instruments, sound effects and songs, and the top-down kit already plays effects for its
events.

Hear it all in the `jukebox` cart: `kartu web carts/jukebox`.

## From Lua

| call | what |
|---|---|
| `sfx(name [, {vol=0..2, pitch=semitones}])` | play a sound effect. Several can overlap; the oldest is cut if all voices are busy |
| `music(name [, {vol=, loop=bool}])` | start a song (loops unless the song says `once`). Calling it again with the song already playing does nothing, so it's safe every frame |
| `music(nil [, {fade=secs}])` | stop the music, optionally fading out |
| `music_playing()` | name of the song playing, or nil |
| `play(mml [, {inst="bell", vol=0.7}])` | play a short MML tune as a sound effect: `play("o5 l16 c e g >c4", {inst="bell"})` |
| `has(name, "sfx"\|"song"\|"instrument")` | does it exist? |

## Built in

**Sound effects:** `coin pickup key heal powerup jump hit hurt swing laser explosion blip select
door locked talk step magic win lose start`

**Songs:** `adventure` (bright, loops) · `village` (gentle) · `cave` (slow, minor) · `boss` (fast,
tense) · `victory` (a short fanfare, plays once)

**Instruments:** `piano epiano bell marimba organ flute lead softlead strings pad brass pluck harp
bass synbass pluckbass square pulse triangle saw sine noise` and the drums `kick snare hat
openhat clap tom crash`

`kartu docs sound_songs` prints the built-in songs, instruments and effects as text — the best
starting point for your own.

## Your own sounds

Three keywords. Sounds have their own namespace (a `coin` sfx and a `coin` sprite are fine).
Declare instruments before the songs that use them.

```text
instrument buzz from=lead duty=0.25 vibrato=0.3,6    -- start from a built-in, change a few things
instrument twang wave=pluck decay=0.6 lowpass=0.5
sfx zap wave=saw from=C7 to=C4 len=0.2 vol=0.6       -- a pitch sweep
sfx chime notes="o6 l16 c e g >c4" inst=bell         -- or a little tune
song theme bpm=120                                   -- then one line per channel (max 6), indented
  lead  @lead o5 l8 [e d c d e e e4]2 | d d d4 e g g4
  bass  @bass o3 l4 c g c g c g c g
  drums @drums [k8 h8 s8 h8]4
```

### Instruments

`wave=square|triangle|saw|sine|noise|pluck|fm` · `duty=0.5` (square) · `attack decay sustain
release` (seconds; sustain 0..1) · `vol` · `vibrato=depth,rate[,delay]` (semitones, Hz, s) ·
`slide=semitones,secs` (start off-pitch and glide in) · `lowpass=0..1` (1 = bright, 0.2 = soft) ·
`detune=semitones` (a second, detuned oscillator: fuller) · `fm=ratio,index[,decay]` (bells,
e-pianos) · `noise=0..1` (mix noise in) · `transpose` · `note=C2` (fixed pitch: drums) ·
`from=name` (start from another instrument).

`pluck` is Karplus–Strong string synthesis; `fm` is 2-operator FM.

### Sound effects

**Sweeps:** `wave from to len vol duty vibrato noise lowpass env=decay|flat|swell`, where `from`
and `to` are notes (`C5`, `F#3`) or Hz. **Tunes:** `notes="MML" inst= bpm=150 vol=`.

### Songs

`song name bpm=N [once] [vol=0.8]`, then indented channel lines: `<channel> @instrument <MML>`.
Channel lines with the same name join up, so a long part can span lines. The song loops at the
end of its **longest** channel: make every channel the same length (`check` warns if not and
prints each song's length in bars).

### MML

`c d e f g a b` with `#`/`+` sharp and `-` flat, then an optional length (`4` quarter, `8`
eighth, `2` half, `1` whole, `16`, `3`/`6`/`12` triplets; `.` dotted) · `r` rest · `o4` octave
(o4 c = middle C), `<` `>` down/up an octave · `l8` default length · `v0`–`v15` volume (12) ·
`q1`–`q8` how much of each note sounds (7 = slightly detached, 8 = legato) · `^8` tie · `@name`
instrument · `[ … ]3` repeat (default 2) · `|` ignored (bar lines, for reading).

In a `@drums` channel the letters are drums: `k` kick, `s` snare, `h` hat, `o` open hat, `c`
clap, `t` tom, `x` crash, `r` rest, with lengths as usual (`k8 h8 s8 h8`).

### Writing music that sounds good

- Melodies in o4–o6, bass in o2–o3.
- Give each channel a role: melody, chords/arpeggio, bass, drums.
- Make every bar add up (4 quarters = `l8` × 8).
- Build from a four-chord loop (C–G–Am–F, or Am–F–C–G for sad) with the bass on the roots;
  lower the accompaniment (`v8`).
- Copy a built-in song as a starting point (`kartu docs sound_songs`).

## Hearing it

- **Web player:** the 🔊 button; sound starts on the first input (browsers require it).
- **Runner:** `kartu run <cart> --sounds` prints `[sound fN] sfx coin` / `music theme` lines;
  `--wav out.wav` writes the sound of the whole run. The summary line `audio <hash> …` reports
  the peak level and clipped samples when anything played.
- **One sound on its own:** `kartu sound <cart> --song NAME | --sfx NAME [--secs N] [--wav F]`;
  with no name it lists everything the cart can play.
- **Handhelds:** the framebuffer player writes to OSS `/dev/dsp` (`--audio DEV`, `--mute`).

## Under the hood

`core/src/audio.rs`. Plain IEEE arithmetic plus the console's own `exp2` and `sin`
polynomials, so the samples — and therefore the audio hash — are bit-identical on x86, ARM and
WebAssembly. ADSR envelopes, a DC blocker and a soft knee on the mix. The web player feeds a
ScriptProcessor from a ring buffer (≤150 ms).
