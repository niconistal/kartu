# Contributing to Kartu

Thank you for being here. Kartu is small and young, which means almost everything about it can
still be improved by one person with an afternoon. This page tells you how.

## Ways to help

- **Play and report.** Try the carts in the [web player](https://niconistal.github.io/kartu/play/)
  or on a handheld. Anything confusing — a message, a crash, a doc that didn't answer your
  question — is worth an [issue](https://github.com/niconistal/kartu/issues/new/choose).
- **Make a cart.** By hand or with an agent. Send it as a pull request into `carts/` with a
  `bot.txt` (or a `win.txt` replay) so `scripts/verify-carts.sh` can prove it's winnable. The
  starter carts are also the art library every new game copies from, so good art helps everyone.
- **Point an AI at it.** Use `kartu new` with your favourite agent and tell us where it
  stumbled. The tool that trips a model up is the one we most want to fix.
- **Improve the core.** Kits (platformer! shmup!), the synth, the bots, the top-down kit's
  defaults, a desktop window player, a port to another handheld.
- **Write.** Guides, examples, a clearer sentence in `API.md` or `AGENTS.md`. The docs are read
  by people *and* by models, so plain words with an example beat clever ones.

If you're planning something big, open an issue first so we can talk about the shape of it.

## Setting up

```sh
git clone https://github.com/niconistal/kartu && cd kartu
cargo build --release
cargo test -p kartu-core
scripts/verify-carts.sh          # every cart checks and wins
```

Rust 1.85+. The web player and the handheld build need extra toolchains; see
[docs/building.md](docs/building.md). You don't need them for most changes.

## Making a change

1. Branch from `main`.
2. Keep the change focused. Small pull requests get reviewed quickly.
3. If you touch the console (`core/`): add or update a test, and run `scripts/verify-carts.sh`.
   The carts are our integration tests; a frame hash that changes means behaviour changed —
   sometimes that's the point, so say so in the PR.
4. If you touch the console API or `assets.cw`: update `API.md` (it's what `kartu docs` and the
   MCP `docs` tool serve) and, if it matters to agents, `maker/skill/AGENTS.md`.
5. If you add a cart: a `-- title:` line in `main.lua`, `bot.txt` or `win.txt`, and make sure
   `kartu check` is clean.
6. Write a commit message that says *why*. Then open a pull request; the template asks for the
   few things we need.

CI runs `cargo build`, `cargo test` and `verify-carts.sh` on every pull request.

## Style

Match the code around you. In Rust that means small modules with a comment at the top saying
what they're for, and error strings that name the file and line when there is one. In Lua kits,
one config table with defaults for everything. In docs, an example before an explanation.

Everything in `core/`, `player/` and `carts/` is MIT; `maker/` is AGPL-3.0. By contributing you
agree that your contribution is licensed the same way as the part it lands in.

## Determinism is a feature

Same seed + same inputs must give the same frame and audio hashes on x86, ARM and WASM. Please
don't reach for `f32::sin`, wall clocks, hash-map iteration order or anything else that varies
between platforms inside the core. If you're not sure, ask — it's the one invariant we're
strict about.

## Getting help

Open a [discussion or issue](https://github.com/niconistal/kartu/issues). There are no silly
questions; if the docs didn't answer it, that's a docs bug.

## Code of conduct

We follow the [Contributor Covenant](CODE_OF_CONDUCT.md). Be kind, assume good faith, and help
newcomers feel like they belong here — because they do.
