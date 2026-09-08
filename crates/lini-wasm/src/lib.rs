//! Lini's compiler, exposed to JavaScript.
//!
//! A binding layer and nothing else: every function here forwards to the
//! library's own entry point ([`lini::compile_str_with`], [`lini::desugar_source`],
//! [`lini::diagnostics_json`], [`lini::highlight_html`]), so a browser runs the
//! *same* engine as the binary — byte for byte, guarded by `tests/wasm.rs`. No
//! compiler logic lives in this crate, and none may: a second lowering path —
//! or a second tokenizer — is exactly the drift the byte-equality test exists
//! to catch.

use lini::{Options, OutputFormat};
use std::collections::BTreeMap;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

/// The host's asset table [SPEC 7], read off a plain JS object.
///
/// A browser has no filesystem, so an `|image| src:` naming a local file can
/// never be opened — the host supplies the bytes instead, keyed by the `src:`
/// exactly as written. Values may be a string (an SVG's text) or a
/// `Uint8Array` (a raster's bytes); anything else is skipped rather than
/// refused, because a host that over-supplies has done nothing wrong and a
/// genuinely missing asset is still reported by the compiler, at the span that
/// asked for it.
///
/// `undefined` — what JavaScript passes when the argument is omitted — is an
/// empty table, which is exactly the old behaviour.
fn assets_from(js: &JsValue) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let Some(obj) = js.dyn_ref::<js_sys::Object>() else {
        return out;
    };
    for entry in js_sys::Object::entries(obj).iter() {
        let pair = js_sys::Array::from(&entry);
        let Some(key) = pair.get(0).as_string() else {
            continue;
        };
        let value = pair.get(1);
        if let Some(text) = value.as_string() {
            out.insert(key, text.into_bytes());
        } else if let Some(bytes) = value.dyn_ref::<js_sys::Uint8Array>() {
            out.insert(key, bytes.to_vec());
        }
    }
    out
}

/// The options every entry point starts from: the caller's asset table, and
/// otherwise the library's defaults.
fn opts_with(assets: &JsValue) -> Options {
    Options {
        assets: assets_from(assets),
        ..Options::default()
    }
}

/// Compile Lini source to SVG.
///
/// Throws on any error-level diagnostic, with the compiler's own LSP-shaped
/// message (`play.lini:3:5: error: …`) as the thrown value.
#[wasm_bindgen]
pub fn compile(src: &str, assets: JsValue) -> Result<String, JsError> {
    lini::compile_str_with(src, &opts_with(&assets)).map_err(js_err)
}

/// Compile to a full HTML page rather than a bare SVG — the `--format html`
/// output, for a preview pane that wants a self-contained document.
#[wasm_bindgen]
pub fn compile_html(src: &str, assets: JsValue) -> Result<String, JsError> {
    let opts = Options {
        format: OutputFormat::Html,
        ..opts_with(&assets)
    };
    lini::compile_str_with(src, &opts).map_err(js_err)
}

/// Compile with `var()` references inlined and text outlined to paths — the
/// `--static` output. Self-contained for download, or for a canvas rasteriser.
#[wasm_bindgen]
pub fn compile_static(src: &str, assets: JsValue) -> Result<String, JsError> {
    let opts = Options {
        static_mode: true,
        ..opts_with(&assets)
    };
    lini::compile_str_with(src, &opts).map_err(js_err)
}

/// Every diagnostic as the JSON document `--json` emits — stable codes, spans,
/// severities, and machine-applicable fixes. Never throws: a file that cannot
/// compile still reports why, which is what an editor's gutter wants.
#[wasm_bindgen]
pub fn diagnostics(src: &str, assets: JsValue) -> String {
    lini::diagnostics_json(src, &opts_with(&assets), "play.lini").0
}

/// The source with every bit of sugar lowered to primitives — what `lini
/// desugar` prints. The teaching view.
#[wasm_bindgen]
pub fn desugar(src: &str) -> Result<String, JsError> {
    lini::desugar_source(src).map_err(js_err)
}

/// Canonical formatting — what `lini fmt` writes.
#[wasm_bindgen]
pub fn format(src: &str) -> Result<String, JsError> {
    lini::format_source(src).map_err(js_err)
}

/// The source as `<span class="lini-tok-…">` HTML — what `lini highlight`
/// prints, from the same scanner the VS Code and Zed grammars take their words
/// from [SPEC 22]. Never throws: highlighting is lexical, so a file
/// mid-keystroke still colours, which is exactly what a live editor wants.
#[wasm_bindgen]
pub fn highlight(src: &str) -> String {
    lini::highlight_html(src)
}

/// The stylesheet [`highlight`]'s markup wears — what `lini highlight --css`
/// prints [SPEC 18/20].
///
/// A host with no `lini` binary beside it — a bundler plugin, an Astro or
/// Docusaurus integration — has no other way to reach the palette, and a copy
/// kept in its own tree goes monochrome the day the scanner names a class this
/// sheet does not paint. Shipping it from the module ties the two to one
/// engine, exactly as [`highlight`] and the grammars already are.
#[wasm_bindgen]
pub fn highlight_css() -> String {
    lini::highlight_css()
}

/// The compiler's version, so a page can show which engine it is running.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn js_err(e: lini::Error) -> JsError {
    JsError::new(&e.to_string())
}
