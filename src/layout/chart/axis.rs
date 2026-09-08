//! Axis rendering [SPEC 14.4]: gridlines (drawn behind the data), then the
//! tick labels and axis titles. The primary value axis and a non-categorical x axis
//! draw gridlines by default; `gridlines: none | colour` overrides per axis.
//!
//! Both cartesian directions run through one pass: every tick projects through
//! [`Plot`] (`value_at` / `domain_at`), so a `direction: row` chart draws the same
//! ticks, gridlines, and titles as a column one with the two screen axes swapped
//! [SPEC 14.7] — never a second, thinner renderer.

use super::metrics::TEXT_SIZE;
use super::model::{Chart, Grid, Side, ValueAxis};
use super::project::{Dir, Plot};
use super::scale::{self, Scale};
use super::text::{self, TEXT};
use super::tint::muted;
use crate::layout::PlacedNode;
use crate::layout::prim;
use crate::resolve::ResolvedValue;

/// All gridlines, drawn first so the data sits over them. A tick's line crosses the
/// plot **perpendicular** to its own axis, so the direction only decides which screen
/// axis each one runs along.
pub fn gridlines(plot: &Plot, chart: &Chart, out: &mut Vec<PlacedNode>) {
    let value_horizontal = plot.dir.value_horizontal();
    for axis in &chart.values {
        if let Some(color) = value_grid(axis) {
            for &t in axis.scale.ticks() {
                let p = plot.value_at(&axis.scale, t);
                out.push(prim::line(
                    plot.cross(value_horizontal, p),
                    color.clone(),
                    1.0,
                ));
            }
        }
    }
    if let Some(color) = x_grid(&chart.x.grid, &chart.x.scale) {
        for &t in chart.x.scale.ticks() {
            let p = plot.domain_at(&chart.x.scale, t);
            out.push(prim::line(
                plot.cross(!value_horizontal, p),
                color.clone(),
                1.0,
            ));
        }
    }
}

/// Tick labels and axis titles for every axis.
pub fn labels(plot: &Plot, chart: &Chart, out: &mut Vec<PlacedNode>) {
    for axis in &chart.values {
        value_labels(plot, axis, chart, out);
    }
    domain_labels(plot, chart, out);
}

/// One value axis's tick labels and title, at the screen edge its `side:` names —
/// honoured as written [SPEC 14.7]. A column chart's value axis runs down a side
/// (left / right), a row chart's along an edge (bottom / top); `model::build` fixes
/// which pair a direction can reach.
fn value_labels(plot: &Plot, axis: &ValueAxis, chart: &Chart, out: &mut Vec<PlacedNode>) {
    let kind = chart.font_kind;
    let cl = chart.clearance;
    let top = matches!(axis.side, Side::Top);
    for &t in axis.scale.ticks() {
        let p = plot.value_at(&axis.scale, t);
        let label = scale::label(&axis.scale, t, axis.fmt, &axis.unit);
        out.push(match (plot.dir, &axis.side) {
            (Dir::Row, _) => text::centered(
                &label,
                p,
                edge_row(plot, top, tick_row(cl)),
                TEXT,
                Some(muted()),
                kind,
            ),
            (_, Side::Right) => text::left(&label, plot.x1 + cl, p, TEXT, Some(muted()), kind),
            _ => text::right(&label, plot.x0 - cl, p, TEXT, Some(muted()), kind),
        });
    }
    // The title sits past the ticks: outside the axis edge in a row, above the plot
    // (aligned to its side) in a column. Either way it stands `clearance` off
    // the ink before it — the plot edge, or the tick row [SPEC 14.6].
    if let Some(title) = &axis.title {
        out.push(match (plot.dir, &axis.side) {
            (Dir::Row, _) => text::centered(
                title,
                plot.center().0,
                edge_row(plot, top, tick_band(cl) + tick_row(cl)),
                TEXT,
                Some(muted()),
                kind,
            ),
            (_, Side::Right) => {
                text::right(title, plot.x1, plot.y0 - cl, TEXT, Some(muted()), kind)
            }
            _ => text::left(title, plot.x0, plot.y0 - cl, TEXT, Some(muted()), kind),
        });
    }
}

/// A tick row's **centre**, measured out from the plot edge it labels: the
/// `clearance` daylight, then half a cap height [SPEC 14.6].
pub(super) fn tick_row(clearance: f64) -> f64 {
    clearance + TEXT_SIZE * 0.7
}

/// …and the whole band one occupies — the daylight plus the row's line box.
/// The one number the placement and the plot-rect reserve ([`super::frame`])
/// both read, so a text always lands in what was set aside for it.
pub(super) fn tick_band(clearance: f64) -> f64 {
    clearance + TEXT_SIZE * 1.4
}

/// The y of a text row `d` outside the plot's top or bottom edge.
fn edge_row(plot: &Plot, top: bool, d: f64) -> f64 {
    if top { plot.y0 - d } else { plot.y1 + d }
}

/// The domain (x) axis's tick labels and title: under the plot in a column chart,
/// down its left in a row — the same texts either way ([`domain_ticks`]).
fn domain_labels(plot: &Plot, chart: &Chart, out: &mut Vec<PlacedNode>) {
    let kind = chart.font_kind;
    let cl = chart.clearance;
    let row = plot.dir == Dir::Row;
    for (v, label) in domain_ticks(chart) {
        let p = plot.domain_at(&chart.x.scale, v);
        out.push(if row {
            text::right(&label, plot.x0 - cl, p, TEXT, Some(muted()), kind)
        } else {
            text::centered(&label, p, plot.y1 + tick_row(cl), TEXT, Some(muted()), kind)
        });
    }
    if let Some(t) = &chart.x.title {
        // A column seats it under the tick row (clear of any band names); a row seats
        // it over the plot's top-left, past whatever value band sits there — the
        // transpose of a column chart's value-axis title.
        out.push(if row {
            text::left(
                t,
                plot.x0,
                plot.y0 - cl - super::frame::value_band(chart, true),
                TEXT,
                Some(muted()),
                kind,
            )
        } else {
            text::centered(
                t,
                plot.center().0,
                plot.y1 + tick_band(cl) + super::annot::x_band_row(chart) + tick_row(cl),
                TEXT,
                Some(muted()),
                kind,
            )
        });
    }
}

/// The domain axis's labels — each one's data coordinate and text [SPEC 14.4]: a
/// band's category slots (its `categories:` entry, else the 1…N index), or a numeric /
/// time scale's tick values formatted by the axis. The one source for the labels and
/// for the gutter that reserves room for them ([`super::frame`]).
pub(super) fn domain_ticks(chart: &Chart) -> Vec<(f64, String)> {
    match &chart.x.scale {
        Scale::Band { n } => (0..*n)
            .map(|i| {
                let label = chart
                    .x
                    .labels
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| (i + 1).to_string());
                (i as f64, label)
            })
            .collect(),
        s => s
            .ticks()
            .iter()
            .map(|&t| (t, scale::label(s, t, chart.x.fmt, &chart.x.unit)))
            .collect(),
    }
}

/// The gridline colour for a value axis: an explicit tint, the faint default for the
/// primary axis, else none (a secondary axis adds none, avoiding moiré — [SPEC 5]).
fn value_grid(axis: &ValueAxis) -> Option<ResolvedValue> {
    match &axis.grid {
        Grid::Color(c) => Some(c.clone()),
        Grid::Off => None,
        Grid::Default => axis.primary.then(|| ResolvedValue::live("grid")),
    }
}

/// The gridline colour for the x axis: an explicit tint, the faint default for any
/// tick-bearing domain — linear, log, or time alike [SPEC 14.4] — none for a
/// categorical band (whose slots carry no ticks).
fn x_grid(grid: &Grid, scale: &Scale) -> Option<ResolvedValue> {
    match grid {
        Grid::Color(c) => Some(c.clone()),
        Grid::Off => None,
        Grid::Default => {
            (!matches!(scale, Scale::Band { .. })).then(|| ResolvedValue::live("grid"))
        }
    }
}
