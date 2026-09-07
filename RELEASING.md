# Releasing

Several repositories ship as one product, and they are not independent:

```
lini              the compiler and the language        crates.io
  ├─ mdbook-lini  links lini as a library              crates.io
  └─ lini-website   lini.rs — builds against the rest  deployed
```

The site also builds against a book theme checked out beside these.

`lini-website` builds the book with **the sibling checkouts**, never with what
crates.io last published, so a fix landed this morning is on the site this
afternoon. That is the whole reason the order below matters, and the source of
the one trap worth knowing.

## The order

1. **lini** — bump, tag, publish, write the release note.
2. **mdbook-lini** — bump, publish. Its `lini = "1.x"` picks up the new
   compiler on the way past.
3. **lini.rs** — `./deploy.sh --prod`.

`mdbook-lini` before `lini` also works, since a caret dependency resolves to
the newest patch at install time. Doing it after means nobody installs the
preprocessor in the window where it would pull the older compiler.

## The trap: a patch that is quietly declined

`deploy.sh` builds the preprocessor with the sibling compiler patched in:

```sh
cargo build --release --manifest-path "$MDBOOK_LINI/Cargo.toml" \
  --config "patch.crates-io.lini.path=\"$LINI_REPO\""
```

**Cargo refuses a patch whose version does not match the lockfile, and says so
in a warning.** Bumping lini to a new version is exactly what breaks the match:
`mdbook-lini/Cargo.lock` still pins the old one, the patch is dropped, and the
book is built by whatever crates.io last published — silently, because a
warning is not a failure.

This has already shipped a page of stale figures and one red error box to
production. Two guards now stand in the way, both in `deploy.sh`:

- the preprocessor step greps its own output for `was not used in the crate
  graph` and stops, naming the fix;
- the site step stops on any `mdbook-lini:` error **or** warning, because a
  figure that fails to draw becomes a visible error box rather than a failed
  build — the right call while writing, the wrong one before a deploy.

The fix, when the first guard fires:

```sh
(cd ../mdbook-lini && cargo update -p lini)
```

Commit that lockfile. It is what `cargo install --locked` will use, so a stale
one hands people the wrong compiler too.

## Before publishing anything

```sh
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
cargo publish --dry-run
```

And sweep for links that a rename just broke — a chapter's URL comes from its
`##` heading in `SPEC.md`, so renaming a section moves a published page:

```sh
grep -rn "docs/reference/" README.md SKILL.md ../mdbook-lini/README.md
```

A README on crates.io is immutable once published. A dead link there outlives
the mistake by a version.

## The release note

`gh release create vX.Y.Z --title "Lini X.Y.Z" --notes-file …`, following the
shape 1.0.0 set: what it is, `cargo install lini`, then a **`### Since`**
section of one bullet per change — what it does for a reader, not what the
commit touched. Link the spec section a fix belongs to.

There is no `CHANGELOG.md`. The releases page is the changelog, and `git log`
is the detail under it.
