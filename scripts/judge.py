#!/usr/bin/env python3
"""Judge a cart from a bot playthrough.

    scripts/judge.py carts/crypt [--plan carts/crypt/bot.txt] [--win WIN] [--frames 20000]
                     [--watch "hearts;hero.x"] [--out verdict.json] [--no-judge]

1. Runs `kartu run <cart> --bot <plan> --until <win>` and parses what happened.
2. Hard facts are decided here, mechanically: won, win frame, bot stuck / no path, cart errors.
3. Judgement calls (was it softlocked? unfair? how hard?) are left to an optional judge command:
   `judge = "cmd"` in ~/.config/kartu/config.toml or $KARTU_JUDGE. It gets {"cart", "facts",
   "trace"} as JSON on stdin and prints a JSON object, reported as `judge`. Without one (or with
   --no-judge) only the mechanical facts are reported.

Prints a one-screen summary and the verdict JSON; exit 0 if the bot won, 1 if not.
"""
import argparse, json, os, re, shutil, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CW = os.environ.get("KARTU_BIN") or (os.path.join(ROOT, "target/release/kartu")
                                     if os.path.exists(os.path.join(ROOT, "target/release/kartu")) else shutil.which("kartu") or "kartu")


def _config_judge():
    try:
        p = subprocess.run([CW, "config", "get", "judge"], capture_output=True, text=True, timeout=10)
        return p.stdout.strip() if p.returncode == 0 else None
    except OSError:
        return None


JUDGE = os.environ.get("KARTU_JUDGE") or _config_judge()

def run(cart, plan, win, frames, watch):
    cmd = [CW, "run", cart, "--bot", plan, "--until", win, "--frames", str(frames)]
    if watch:
        cmd += ["--watch", watch, "--watch-every", "120"]
    p = subprocess.run(cmd, capture_output=True, text=True)
    return p.returncode, p.stdout + p.stderr


def parse(code, out, win):
    facts = {"exit": code, "won": False, "win_frame": None, "bot_failure": None, "cart_error": None,
             "plan_done": "plan done" in out}
    m = re.search(r"until: `.*` at f(\d+)", out)
    if m:
        facts["won"], facts["win_frame"] = True, int(m.group(1))
    m = re.search(r"\[bot f\d+\] (FAILED.*)", out)
    if m:
        facts["bot_failure"] = m.group(1)
    m = re.search(r"^error at f(\d+): (.*)", out, re.M)
    if m:
        facts["cart_error"] = f"f{m.group(1)}: {m.group(2)}"
    m = re.search(r"^frames (\d+)", out, re.M)
    facts["frames"] = int(m.group(1)) if m else None
    # hits before anyone could react: within 1 s of boot, play starting or entering a room
    # (kit log conventions: `state play`, `room X`, `hurt ...`/`ouch ...`)
    arrived, early = 0, []
    for f, msg in re.findall(r"^\[log f(\d+)\] (.*)", out, re.M):
        f = int(f)
        if msg.startswith(("room ", "state play")):
            arrived = f
        elif re.match(r"(hurt|ouch|hit)\b", msg) and f - arrived < 60:
            early.append(f"f{f} ({(f - arrived) / 60:.2f} s after arriving)")
    facts["early_hits"] = early
    return facts


def trace(out, limit=70):
    """The lines worth reading, consecutive repeats folded."""
    keep, last, rep = [], None, 0
    for l in out.splitlines():
        if not l.startswith(("[log", "[bot", "[watch", "error at", "until:")):
            continue
        body = re.sub(r"^\[\w+ f\d+\] ", "", l)
        # frames -> seconds, which is what "could a human react?" is about
        l = re.sub(r"^\[(\w+) f(\d+)\]", lambda m: f"[{m.group(1)} {int(m.group(2)) / 60:6.2f}s]", l)
        if body == last:
            rep += 1
            continue
        if rep:
            keep.append(f"  (repeated {rep} more times)")
        keep.append(l)
        last, rep = body, 0
    if len(keep) > limit:
        keep = keep[: limit // 2] + [f"  … {len(keep) - limit} lines …"] + keep[-limit // 2:]
    return "\n".join(keep)


def ask_judge(cart, facts, tr):
    """The optional judge: a command that reads {"cart", "facts", "trace"} as JSON on stdin and
    prints a JSON object of opinions (e.g. softlocked / unfair / difficulty)."""
    if not JUDGE:
        return None
    p = subprocess.run(JUDGE, shell=True, input=json.dumps({"cart": os.path.abspath(cart), "facts": facts, "trace": tr}),
                       capture_output=True, text=True, timeout=180)
    if p.returncode != 0:
        return {"error": (p.stderr.strip().splitlines() or ["judge failed"])[-1][:200]}
    return json.loads(p.stdout)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("cart")
    ap.add_argument("--plan")
    ap.add_argument("--win", default="WIN")
    ap.add_argument("--frames", type=int, default=20000)
    ap.add_argument("--watch", default="")
    ap.add_argument("--out")
    ap.add_argument("--no-judge", action="store_true")
    a = ap.parse_args()
    plan = a.plan or os.path.join(a.cart, "bot.txt")
    if not os.path.exists(plan):
        sys.exit(f"judge: no plan at {plan} (write one: see API.md, Bots)")
    code, out = run(a.cart, plan, a.win, a.frames, a.watch)
    facts = parse(code, out, a.win)
    tr = trace(out)
    verdict = {"cart": a.cart, "plan": plan, **facts}
    if not a.no_judge:
        j = ask_judge(a.cart, facts, tr)
        if j is not None:
            verdict["judge"] = j
    print(tr)
    print("-" * 60)
    won = f"WON at f{facts['win_frame']} ({facts['win_frame'] / 60:.1f} s)" if facts["won"] else "NOT won"
    print(f"{a.cart}: {won}" + (f" · bot: {facts['bot_failure']}" if facts["bot_failure"] else "")
          + (f" · EARLY HITS {', '.join(facts['early_hits'])}" if facts["early_hits"] else "")
          + (f" · cart error {facts['cart_error']}" if facts["cart_error"] else ""))
    j = verdict.get("judge")
    if j:
        print("judge: " + (f"unavailable ({j['error']})" if "error" in j else " · ".join(f"{k} {v}" for k, v in j.items())))
    print(json.dumps(verdict))
    if a.out:
        with open(a.out, "w") as f:
            json.dump(verdict, f, indent=1)
    sys.exit(0 if facts["won"] else 1)


if __name__ == "__main__":
    main()
