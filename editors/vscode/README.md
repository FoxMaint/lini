<p align="center">
  <a href="https://lini.rs"><img src="https://raw.githubusercontent.com/monfa-red/lini/main/assets/logo/lini.svg" alt="Lini" width="200"></a>
</p>

<p align="center"><strong>From mindmap to blueprint, in your editor.</strong></p>

<p align="center"><a href="https://lini.rs">lini.rs</a> — the tour, the reference, a playground in the browser, and the gallery.</p>

# Lini for VS Code

Syntax highlighting for [Lini](https://lini.rs) (`.lini`) — one small language
for every kind of figure: diagrams, mindmaps, charts, sequences, ER schemas,
engineering drawings, floor plans and circuit schematics, compiled to clean,
themeable SVG.

```lini
cat -> dog -> bird
```

Highlights comments, strings, numbers, `|type#id|` identity bars, `.class`,
`#id`, `--var` references and `--var:` declarations, `name = value` bindings,
the link and measuring operators (`->`, `<->`, `~>`, `(-)`, `(o)`, `(<)`,
`>-`, `||`), property names (generated from the compiler's own property
ledger, so they never drift), value builders (`gradient(`, `oklch(`,
`repeat(`, …), and `( )` math expressions.

Install the `lini` CLI to compile: `cargo install lini` — or skip it and try
the [playground](https://lini.rs/play/) in the browser. The
[tour](https://lini.rs/docs/), the [reference](https://lini.rs/docs/reference/00-quickstart.html)
and the [gallery](https://lini.rs/gallery/) are at [lini.rs](https://lini.rs);
the [repository](https://github.com/monfa-red/lini) has the samples.
