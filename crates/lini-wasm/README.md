# lini-wasm

[Lini](https://lini.rs) compiles plain text to clean, themeable SVG — one small
language for flowcharts, mindmaps, org charts, tables, ER schemas, sequences,
charts, engineering drawings, floor plans, and circuit schematics.

This package is the compiler itself, as a WebAssembly module: the same engine as
the `lini` binary, byte for byte, with no network calls and nothing to stand up
beside it.

```sh
npm install lini-wasm
```

## Node, Bun, Deno

The module is read off disk as it loads, so there is nothing to await.

```js
import { compile } from "lini-wasm";

const svg = compile(`
{ direction: row; gap: 40; }
|box#write| "write"
|box#compile| "compile"
|box#svg| "SVG"
write -> compile -> svg
`);
```

## Browsers and bundlers

Vite, Astro, webpack, or a plain `<script type="module">` — the WebAssembly is
fetched on first use, so `init()` is awaited once:

```js
import init, { compile } from "lini-wasm";

await init();
document.querySelector("#figure").innerHTML = compile('|box| "hello"');
```

Bundlers pick this build up automatically. A page loading the file directly
imports `lini-wasm/web` and passes the module's URL:

```js
import init, { compile } from "https://esm.sh/lini-wasm/web";
await init("https://esm.sh/lini-wasm/lini_wasm_bg.wasm");
```

## What it exports

| | |
|---|---|
| `compile(src)` | SVG — the default output, live `var()` colours and real `<text>` |
| `compile_html(src)` | a self-contained HTML page around that SVG |
| `compile_static(src)` | SVG with variables inlined and text outlined, for a rasteriser |
| `diagnostics(src)` | every error and warning as JSON — codes, spans, fixes. Never throws |
| `desugar(src)` | the source with all sugar lowered to primitives |
| `format(src)` | canonical formatting, what `lini fmt` writes |
| `highlight(src)` | the source as `<span class="lini-tok-…">` HTML |
| `highlight_css()` | the palette those spans wear, ready to ship beside them |
| `version()` | the engine's version |

Everything but `diagnostics`, `highlight` and `highlight_css` throws on an error-level
diagnostic, with the compiler's own `play.lini:3:5: error: …` message as the
thrown value. TypeScript declarations ship with both builds.

## More

[**lini.rs**](https://lini.rs) — the tour, the full reference, the gallery, and
this compiler running in your browser.
[github.com/monfa-red/lini](https://github.com/monfa-red/lini) — the source, the
CLI, and `SKILL.md`, the whole language as a working brief.

MIT.
