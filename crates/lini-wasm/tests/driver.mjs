// Compile and highlight every sample through the browser artifact and write
// the results where `tests/wasm.rs` can diff them against the binary's own.
//
//   node driver.mjs <pkg-dir> <out-dir> <sample.lini>…
//
// Deliberately the *same* entry points a web page calls — `compile(src, assets)`
// and `highlight(src)` — so the test exercises the shipped path rather than a
// test-only shortcut.

import { readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { pathToFileURL } from "node:url";

// The asset table a browser host has to supply [SPEC 7]: there is no
// filesystem behind `compile`, so every local `|image| src:` is read here and
// handed over by the path exactly as the source wrote it. HTTP(S) URLs and
// authored `data:` URIs pass through the compiler untouched and are skipped.
//
// This is the same job the playground does — read the source, fetch what it
// names, pass the table — which is the point: the driver must exercise the
// path a page takes, not a shortcut only the test knows.
const assetsFor = (path, src) => {
  const table = {};
  for (const [, value] of src.matchAll(/\bsrc:\s*"([^"]*)"/g)) {
    if (/^(https?:|data:)/i.test(value)) continue;
    try {
      table[value] = readFileSync(join(dirname(path), value), "utf8");
    } catch {
      // Leave it out: the compiler reports a missing asset at the span that
      // asked for it, which is a better error than anything invented here.
    }
  }
  return table;
};

const [pkgDir, outDir, ...samples] = process.argv.slice(2);

// The `web` build of the two the package ships — the one a browser and a
// bundler resolve to, and so the one whose bytes the playground serves.
const { default: init, compile, highlight } = await import(
  pathToFileURL(`${pkgDir}/web/lini_wasm.js`).href
);
await init({ module_or_path: readFileSync(`${pkgDir}/web/lini_wasm_bg.wasm`) });

let failed = 0;
for (const [i, path] of samples.entries()) {
  // Keyed by position in the argument list — the one fact the Rust side
  // shares — because basenames collide: the corpus carries a samples/ sheet
  // and a tests/fixtures/routing/ fixture both named links_hard.lini, and a
  // flat name would let the later compile silently overwrite the earlier.
  const name = `${i}-${basename(path).replace(/\.lini$/, "")}`;
  try {
    const src = readFileSync(path, "utf8");
    // Highlighting first: it never throws, so a sample the compiler rejects
    // still reports its listing rather than nothing.
    writeFileSync(`${outDir}/${name}.html`, highlight(src));
    writeFileSync(`${outDir}/${name}.svg`, compile(src, assetsFor(path, src)));
  } catch (e) {
    // A sample the browser build rejects is a real difference — record it as
    // the output so the Rust side reports a diff rather than a missing file.
    writeFileSync(`${outDir}/${name}.svg`, `WASM ERROR: ${e.message ?? e}\n`);
    failed++;
  }
}
if (failed) console.error(`${failed} sample(s) threw in wasm`);
