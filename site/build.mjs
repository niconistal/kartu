// Render the Kartu website into dist/site: the landing page (site/index.html), every guide
// in docs/ and API.md as HTML with a shared shell, plus docs/images. The web player is copied
// in by scripts/build-site.sh, which is what you should run.
import fs from "node:fs";
import path from "node:path";
import { marked } from "marked";
import hljs from "highlight.js";

const ROOT = path.resolve(import.meta.dirname, "..");
const OUT = path.join(ROOT, "dist/site");
const REPO = "https://github.com/niconistal/kartu";
const SITE = "https://niconistal.github.io/kartu";

// Sidebar order. `group` starts a new section.
const PAGES = [
  { file: "docs/README.md", out: "docs/index.html", title: "Overview" },
  { group: "Start here" },
  { file: "docs/getting-started.md", out: "docs/getting-started.html", title: "Getting started" },
  { file: "docs/making-a-game.md", out: "docs/making-a-game.html", title: "Making a game" },
  { file: "docs/cart-format.md", out: "docs/cart-format.html", title: "Cart format" },
  { file: "docs/kits.md", out: "docs/kits.html", title: "Kits" },
  { file: "docs/sound.md", out: "docs/sound.html", title: "Sound" },
  { group: "Tools" },
  { file: "docs/runner.md", out: "docs/runner.html", title: "The runner" },
  { file: "docs/ai-tools.md", out: "docs/ai-tools.html", title: "AI tools" },
  { group: "Under the hood" },
  { file: "docs/console.md", out: "docs/console.html", title: "The console" },
  { file: "docs/handhelds.md", out: "docs/handhelds.html", title: "Handhelds" },
  { file: "docs/building.md", out: "docs/building.html", title: "Building" },
  { group: "Reference" },
  { file: "API.md", out: "docs/api.html", title: "API reference" },
];

const slug = (s) => s.toLowerCase().replace(/<[^>]+>/g, "").replace(/[^\w\s-]/g, "").trim().replace(/\s+/g, "-");

// Turn repo-relative Markdown links into site links; anything outside docs/ + API.md points at GitHub.
function rewrite(href, fromFile) {
  if (/^(https?:|mailto:|#)/.test(href)) return href;
  const [p, anchor] = href.split("#");
  const target = path.normalize(path.join(path.dirname(fromFile), p));
  const hash = anchor ? `#${anchor}` : "";
  if (target === "API.md") return `api.html${hash}`;
  if (target.startsWith("docs/")) {
    const rest = target.slice(5);
    if (rest.startsWith("images/")) return rest;
    if (rest === "README.md") return `index.html${hash}`;
    if (rest.endsWith(".md")) return rest.replace(/\.md$/, ".html") + hash;
    return rest + hash;
  }
  return `${REPO}/blob/main/${target}${hash}`;
}

let currentFile = "";
const seen = new Map();
marked.use({
  gfm: true,
  walkTokens(t) {
    if (t.type === "link" || t.type === "image") t.href = rewrite(t.href, currentFile);
  },
  renderer: {
    heading({ tokens, depth }) {
      const text = this.parser.parseInline(tokens);
      let id = slug(text);
      const n = seen.get(id) || 0; seen.set(id, n + 1); if (n) id += `-${n}`;
      return `<h${depth} id="${id}"><a class="anchor" href="#${id}" aria-hidden="true">#</a>${text}</h${depth}>\n`;
    },
    code({ text, lang }) {
      const l = (lang || "").split(/\s/)[0];
      const known = l && hljs.getLanguage(l);
      const body = known ? hljs.highlight(text, { language: l }).value : escape(text);
      return `<pre><code class="hljs${l ? ` language-${l}` : ""}">${body}</code></pre>\n`;
    },
    table({ header, rows }) {
      const cell = (c, tag) => `<${tag}${c.align ? ` style="text-align:${c.align}"` : ""}>${this.parser.parseInline(c.tokens)}</${tag}>`;
      const head = `<tr>${header.map((c) => cell(c, "th")).join("")}</tr>`;
      const body = rows.map((r) => `<tr>${r.map((c) => cell(c, "td")).join("")}</tr>`).join("\n");
      return `<div class="table-wrap"><table><thead>${head}</thead><tbody>${body}</tbody></table></div>\n`;
    },
  },
});
const escape = (s) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));

function shell({ title, description, body, sidebar, depth, page }) {
  const r = depth ? "../".repeat(depth) : "";
  const url = `${SITE}/${page}`;
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escape(title)} · Kartu</title>
<meta name="description" content="${escape(description)}">
<meta property="og:title" content="${escape(title)} · Kartu">
<meta property="og:description" content="${escape(description)}">
<meta property="og:image" content="${SITE}/docs/images/social.jpg">
<meta property="og:url" content="${url}">
<meta name="twitter:card" content="summary_large_image">
<link rel="icon" href="${r}favicon.svg" type="image/svg+xml">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Silkscreen:wght@400;700&display=swap">
<link rel="stylesheet" href="${r}style.css">
</head>
<body class="doc">
<header class="top">
  <a class="brand" href="${r}index.html"><span class="mark">▣</span> KARTU</a>
  <nav>
    <a href="${r}docs/index.html" class="on">Docs</a>
    <a href="${r}docs/api.html">API</a>
    <a href="${r}play/">Play</a>
    <a href="${REPO}">GitHub</a>
  </nav>
</header>
<div class="layout">
  <aside class="side">
    <details class="side-menu" open>
      <summary>Documentation</summary>
      ${sidebar}
    </details>
  </aside>
  <main class="content">
${body}
    <footer class="doc-foot">
      <a href="${REPO}/edit/main/${escape(currentFile)}">Edit this page on GitHub</a> ·
      <a href="${REPO}/issues/new/choose">Something unclear? Tell us</a>
    </footer>
  </main>
</div>
<script>
// remember the sidebar state on phones
const d = document.querySelector('.side-menu');
if (matchMedia('(max-width: 900px)').matches) { try { d.open = localStorage.getItem('kartu.side') === '1'; } catch {} }
d.addEventListener('toggle', () => { try { localStorage.setItem('kartu.side', d.open ? '1' : '0'); } catch {} });
</script>
</body>
</html>
`;
}

function sidebarFor(active) {
  let html = "<ul>";
  for (const p of PAGES) {
    if (p.group) { html += `</ul><h4>${p.group}</h4><ul>`; continue; }
    const href = path.basename(p.out);
    html += `<li><a href="${href}"${p.out === active ? ' class="on" aria-current="page"' : ""}>${escape(p.title)}</a></li>`;
  }
  return html + "</ul>";
}

function firstParagraph(md) {
  const m = md.replace(/^#.*$/m, "").match(/^(?!\s*[#|`<>\-*])(.+)$/m);
  return (m ? m[1] : "Kartu, a fantasy console designed to be programmed by AI and by people.").replace(/[*`_\[\]]/g, "").slice(0, 200);
}

fs.rmSync(OUT, { recursive: true, force: true });
fs.mkdirSync(path.join(OUT, "docs/images"), { recursive: true });

for (const p of PAGES) {
  if (p.group) continue;
  currentFile = p.file; seen.clear();
  const md = fs.readFileSync(path.join(ROOT, p.file), "utf8");
  const body = marked.parse(md);
  const html = shell({
    title: p.title, description: firstParagraph(md), body,
    sidebar: sidebarFor(p.out), depth: p.out.split("/").length - 1, page: p.out,
  });
  fs.writeFileSync(path.join(OUT, p.out), html);
  console.log("rendered", p.out);
}

for (const f of fs.readdirSync(path.join(ROOT, "docs/images"))) {
  fs.copyFileSync(path.join(ROOT, "docs/images", f), path.join(OUT, "docs/images", f));
}
for (const f of ["index.html", "style.css", "favicon.svg"]) {
  fs.copyFileSync(path.join(ROOT, "site", f), path.join(OUT, f));
}
fs.writeFileSync(path.join(OUT, ".nojekyll"), "");
console.log("site →", OUT);
