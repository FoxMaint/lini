//! Chart chrome text [SPEC 14.6/18] — every string a chart draws, seated on a
//! `.lini-chart-*` **stylesheet rule** rather than an inline font.
//!
//! One size lives in [`TEXT_SIZE`] and one rule states it per role, so a chart
//! of fifty ticks writes its font once in CSS instead of fifty times in
//! `style=`. The leaf still carries what is genuinely **its own** — a band
//! tick's tint, a mark's stroke colour — which is exactly the class-diff law
//! [SPEC 18]: the rule holds what every wearer shares, the element only its
//! difference. The mirror of a drawing's `.lini-dim-text` ([`prim::dim_text`]).

use super::metrics::TEXT_SIZE;
use crate::font::{Font, Kind};
use crate::layout::PlacedNode;
use crate::layout::prim;
use crate::resolve::ResolvedValue;

/// Data text — tick labels, axis titles, band ticks, mark labels, a radial
/// chart's spoke categories. Normal weight, so the numbers read quietly under
/// the captions [SPEC 14.6].
pub(super) const TEXT: &str = "chart-text";
/// A legend entry — chrome, so semibold beside the title [SPEC 14.6].
pub(super) const LEGEND: &str = "chart-legend";
/// A per-datum inline label [SPEC 14.8] — data text that also takes no pointer
/// events, so hovering a labelled point still reaches the point's card.
pub(super) const LABEL: &str = "chart-label";

/// The weight a role renders at — read once here, so the measured box and the
/// rule's own `font-weight` can never disagree.
pub(super) fn font(class: &str, kind: Kind) -> Font {
    if class == LEGEND {
        Font::semibold(kind)
    } else {
        Font::regular(kind)
    }
}

/// A chart text centred at (cx, cy), wearing `class`; `color` is the leaf's own
/// diff (`None` inherits the chart's).
pub(super) fn centered(
    content: &str,
    cx: f64,
    cy: f64,
    class: &str,
    color: Option<ResolvedValue>,
    kind: Kind,
) -> PlacedNode {
    let mut n = prim::text_classed(content, cx, cy, TEXT_SIZE, class, font(class, kind));
    if let Some(c) = color {
        prim::set_color(&mut n, c);
    }
    n
}

/// …with its **right edge** at `right_x` (a left-hand value axis's ticks).
pub(super) fn right(
    content: &str,
    right_x: f64,
    cy: f64,
    class: &str,
    color: Option<ResolvedValue>,
    kind: Kind,
) -> PlacedNode {
    let cx = right_x - width(content, class, kind) / 2.0;
    centered(content, cx, cy, class, color, kind)
}

/// …with its **left edge** at `left_x` (a right-hand axis's ticks, a title).
pub(super) fn left(
    content: &str,
    left_x: f64,
    cy: f64,
    class: &str,
    color: Option<ResolvedValue>,
    kind: Kind,
) -> PlacedNode {
    let cx = left_x + width(content, class, kind) / 2.0;
    centered(content, cx, cy, class, color, kind)
}

/// The drawn width of a chart text in `class`'s weight — the one measurement
/// the gutters, the legend row, and the right-aligned ticks all read.
pub(super) fn width(content: &str, class: &str, kind: Kind) -> f64 {
    prim::text_width(content, TEXT_SIZE, font(class, kind))
}
