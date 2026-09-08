//! Chart look metrics [SPEC 14] — the type scale and paint constants every
//! chart family shares, in one home.

/// The chart title.
// `pub(crate)`: the `.lini-chart-title` rule derives its px from this, the
// same one-source pattern as `messages::LABEL_SIZE` [SPEC 18].
pub(crate) const TITLE_SIZE: f64 = 15.0;
/// **Every other text a chart draws** — tick labels, axis titles, legend
/// entries, band / mark labels, per-datum labels. One size, because a chart's
/// type scale is two steps: the title, and everything under it [SPEC 14.6].
/// The `.lini-chart-text` / `-legend` / `-label` rules derive their px from
/// this one constant, so no chart leaf inlines a font size and a bump here
/// moves the measured box and the rendered glyph together.
pub(crate) const TEXT_SIZE: f64 = 12.0;
/// An area / radar body's fill opacity, so gridlines and overlaps still read.
pub(super) const AREA_OPACITY: f64 = 0.82;
/// The tick count a "nice" step aims for (`range / TICK_TARGET`).
pub(super) const TICK_TARGET: f64 = 5.0;
