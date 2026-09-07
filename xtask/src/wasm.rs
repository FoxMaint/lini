//! Build the npm package — `cargo xtask wasm`.
//!
//! One compile, two bindings, one package:
//!
//! ```text
//!   cargo build --profile wasm-release --target wasm32-unknown-unknown
//!         │  the compiler, minus the font subsets (see crates/lini-wasm)
//!         ├─ wasm-bindgen --target web    ─→ pkg/web/   browsers, bundlers
//!         └─ wasm-bindgen --target nodejs ─→ pkg/node/  Node, Bun, Deno
//!         │  the JS glue — strings across the boundary, no `unsafe` on our side
//!   wasm-opt -Oz
//!         │  ~10 % off each raw module
//!   package.json + README.md + LICENSE
//!         ▼
//!   crates/lini-wasm/pkg/   —  `npm publish` runs from here
//! ```
//!
//! The two bindings are the same engine with different loaders: `web` is ESM
//! and fetches its module (`await init()`), `nodejs` is CommonJS and reads it
//! off disk, so there is nothing to await. `package.json`'s `"exports"` picks
//! between them by condition, which is the whole reason both are built.
//!
//! `wasm-bindgen`'s CLI must match the `wasm-bindgen` crate version exactly, so
//! the mismatch is reported here rather than as a confusing runtime failure.
//! `wasm-opt` is optional: without it the artifact still works, just larger —
//! CI installs it, a local build need not.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const TARGET: &str = "wasm32-unknown-unknown";
const PROFILE: &str = "wasm-release";

/// `(wasm-bindgen target, directory under pkg/)`. Kept in this order so the
/// size report leads with the build the playground loads.
const BUILDS: [(&str, &str); 2] = [("web", "web"), ("nodejs", "node")];

pub fn build() -> ExitCode {
    let root = match workspace_root() {
        Some(r) => r,
        None => {
            eprintln!("cannot locate the workspace root");
            return ExitCode::FAILURE;
        }
    };
    let out = root.join("crates/lini-wasm/pkg");

    let version = match package_version(&root) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    if !run(
        "cargo",
        &[
            "build",
            "--profile",
            PROFILE,
            "--target",
            TARGET,
            "-p",
            "lini-wasm",
        ],
        &root,
    ) {
        return ExitCode::FAILURE;
    }

    let module = root
        .join("target")
        .join(TARGET)
        .join(PROFILE)
        .join("lini_wasm.wasm");
    if !module.is_file() {
        eprintln!("expected {} after the build", module.display());
        return ExitCode::FAILURE;
    }

    if let Err(e) = check_bindgen_version(&root) {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }

    let mut unoptimized = false;
    for (target, dir) in BUILDS {
        let dir = out.join(dir);
        if !run(
            "wasm-bindgen",
            &[
                "--target",
                target,
                "--out-dir",
                &dir.to_string_lossy(),
                &module.to_string_lossy(),
            ],
            &root,
        ) {
            return ExitCode::FAILURE;
        }
        unoptimized |= !optimize(&dir.join("lini_wasm_bg.wasm"), &root);
    }
    if unoptimized {
        eprintln!("note: wasm-opt not run — the modules are larger than they need to be");
        eprintln!("      install it with `brew install binaryen` (or via npm)");
    }

    if let Err(e) = write_manifest(&root, &out, &version) {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }

    report(&out, &version);
    ExitCode::SUCCESS
}

/// Run `wasm-opt -Oz` in place. Absent, the build still succeeds — the artifact
/// is simply the unoptimized one, and the size report says so.
fn optimize(bg: &Path, cwd: &Path) -> bool {
    let tmp = bg.with_extension("opt");
    let ok = run(
        "wasm-opt",
        &[
            "-Oz",
            // Rust emits both by default on wasm32 now; wasm-opt still gates
            // them, so a plain `-Oz` fails validation without these.
            "--enable-bulk-memory-opt",
            "--enable-nontrapping-float-to-int",
            "--strip-debug",
            "--strip-producers",
            "-o",
            &tmp.to_string_lossy(),
            &bg.to_string_lossy(),
        ],
        cwd,
    );
    if ok {
        let _ = std::fs::rename(&tmp, bg);
    } else {
        let _ = std::fs::remove_file(&tmp);
    }
    ok
}

/// The npm version is the workspace's, so the two can never disagree — and the
/// binding crate must track it, because `version()` in the module reports *its*
/// `CARGO_PKG_VERSION` and a drifted one would have the package lie about the
/// engine inside it.
fn package_version(root: &Path) -> Result<String, String> {
    let workspace = manifest_version(&root.join("Cargo.toml"))?;
    let binding = manifest_version(&root.join("crates/lini-wasm/Cargo.toml"))?;
    if workspace != binding {
        return Err(format!(
            "the workspace is {workspace} but crates/lini-wasm is {binding}\n  \
             fix: set crates/lini-wasm/Cargo.toml's version to {workspace}"
        ));
    }
    Ok(workspace)
}

/// The first `version = "…"` of a manifest — always the `[package]` one, since
/// that table opens the file in both manifests read here.
fn manifest_version(path: &Path) -> Result<String, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    text.lines()
        .find_map(|l| l.strip_prefix("version = "))
        .and_then(|v| v.split('"').nth(1))
        .map(str::to_owned)
        .ok_or_else(|| format!("no version in {}", path.display()))
}

/// Everything in the package that is not a wasm-bindgen output: the manifest
/// that maps the two builds onto runtime conditions, the page npm renders, and
/// the licence a consumer's audit expects to find beside them.
fn write_manifest(root: &Path, out: &Path, version: &str) -> Result<(), String> {
    let write = |path: PathBuf, body: String| -> Result<(), String> {
        std::fs::write(&path, body).map_err(|e| format!("cannot write {}: {e}", path.display()))
    };
    let copy = |from: PathBuf, to: PathBuf| -> Result<(), String> {
        std::fs::copy(&from, &to)
            .map(|_| ())
            .map_err(|e| format!("cannot copy {} → {}: {e}", from.display(), to.display()))
    };

    write(
        out.join("package.json"),
        PACKAGE_JSON.replace("{version}", version),
    )?;
    // `pkg/` is `"type": "module"`, and the nodejs binding is CommonJS: without
    // this scope Node reads its `require` as an ESM file and throws on load.
    write(out.join("node/package.json"), NODE_SCOPE.to_owned())?;
    copy(
        root.join("crates/lini-wasm/README.md"),
        out.join("README.md"),
    )?;
    copy(root.join("LICENSE"), out.join("LICENSE"))
}

/// `"node"` before `"default"` so a runtime that has one takes the CommonJS
/// build and a bundler falls through to the ESM one; `"types"` first in each
/// branch because TypeScript resolves conditions in order and stops.
const PACKAGE_JSON: &str = r#"{
  "name": "lini-wasm",
  "version": "{version}",
  "description": "Lini's compiler as a WebAssembly module — one small language for diagrams, compiled to clean, themeable SVG.",
  "license": "MIT",
  "homepage": "https://lini.rs",
  "repository": {
    "type": "git",
    "url": "git+https://github.com/monfa-red/lini.git",
    "directory": "crates/lini-wasm"
  },
  "bugs": "https://github.com/monfa-red/lini/issues",
  "keywords": [
    "diagram",
    "diagrams-as-code",
    "svg",
    "dsl",
    "wasm",
    "webassembly",
    "lini"
  ],
  "type": "module",
  "types": "./web/lini_wasm.d.ts",
  "main": "./node/lini_wasm.js",
  "module": "./web/lini_wasm.js",
  "browser": "./web/lini_wasm.js",
  "exports": {
    ".": {
      "node": {
        "types": "./node/lini_wasm.d.ts",
        "default": "./node/lini_wasm.js"
      },
      "types": "./web/lini_wasm.d.ts",
      "default": "./web/lini_wasm.js"
    },
    "./web": {
      "types": "./web/lini_wasm.d.ts",
      "default": "./web/lini_wasm.js"
    },
    "./node": {
      "types": "./node/lini_wasm.d.ts",
      "default": "./node/lini_wasm.js"
    },
    "./lini_wasm_bg.wasm": "./web/lini_wasm_bg.wasm",
    "./package.json": "./package.json"
  },
  "files": [
    "web",
    "node",
    "README.md",
    "LICENSE"
  ],
  "engines": {
    "node": ">=18"
  }
}
"#;

const NODE_SCOPE: &str = "{\n  \"type\": \"commonjs\"\n}\n";

fn report(out: &Path, version: &str) {
    eprintln!("wrote {} — lini-wasm {version}", out.display());
    for (_, dir) in BUILDS {
        for name in ["lini_wasm_bg.wasm", "lini_wasm.js", "lini_wasm.d.ts"] {
            if let Ok(m) = std::fs::metadata(out.join(dir).join(name)) {
                eprintln!("  {dir}/{name:<17}  {}", human(m.len() as usize));
            }
        }
    }
    eprintln!("publish it with `npm publish` from {}", out.display());
}

fn human(n: usize) -> String {
    if n >= 1 << 20 {
        format!("{:.2} MB", n as f64 / (1 << 20) as f64)
    } else {
        format!("{:.1} KB", n as f64 / 1024.0)
    }
}

/// The CLI and the crate must agree exactly; a mismatch produces glue that
/// throws on load, which is a miserable thing to debug from the browser.
fn check_bindgen_version(root: &Path) -> Result<(), String> {
    let cli = Command::new("wasm-bindgen")
        .arg("--version")
        .output()
        .map_err(|_| {
            "wasm-bindgen not found — install it with `cargo install wasm-bindgen-cli --version \
             <the version in Cargo.lock>`"
                .to_string()
        })?;
    let cli = String::from_utf8_lossy(&cli.stdout);
    let cli = cli.split_whitespace().nth(1).unwrap_or("").to_string();

    let lock = std::fs::read_to_string(root.join("Cargo.lock")).unwrap_or_default();
    let crate_version = lock
        .split("[[package]]")
        .find(|p| p.contains("name = \"wasm-bindgen\"\n"))
        .and_then(|p| p.lines().find(|l| l.starts_with("version = ")))
        .and_then(|l| l.split('"').nth(1))
        .unwrap_or_default()
        .to_string();

    if crate_version.is_empty() || cli == crate_version {
        Ok(())
    } else {
        Err(format!(
            "wasm-bindgen CLI is {cli}, but the crate is {crate_version}\n  \
             fix: cargo install wasm-bindgen-cli --version {crate_version}"
        ))
    }
}

fn run(cmd: &str, args: &[&str], cwd: &Path) -> bool {
    Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .status()
        .is_ok_and(|s| s.success())
}

fn workspace_root() -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
}
