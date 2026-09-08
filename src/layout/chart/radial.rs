//! Radial (polar) gridlines and labels [SPEC 14.7]. The value axis's concentric
//! polygon **web** through the spokes, the spokes themselves (the domain gridlines),
//! the spoke (category) labels around the rim, and the radius tick labels up the top
//! spoke. The series reuse the cartesian builders through `Plot::project`; only these
//! gridlines and labels are radial-specific.

use super::metrics::TEXT_SIZE;
use super::model::Chart;
use super::project::Plot;
use super::scale::{self, Scale};
use super::text::{self, TEXT};
use crate::layout::PlacedNode;
use crate::layout::prim;
use crate::resolve::ResolvedValue;

/// The web (concentric polygons through the spokes at each radius tick) and the spokes
/// (centre → rim at each domain position), drawn first so the data sits over them.
pub fn gridlines(plot: &Plot, chart: &Chart, out: &mut Vec<PlacedNode>) {
    let Scale::Band { n } = chart.x.scale else {
        return;
    };
    if n == 0 {
        return;
    }
    let xs = &chart.x.scale;
    let vs = &chart.values[0].scale; // a radial chart has one radius (value) axis
    let grid = ResolvedValue::live("grid");
    for &t in vs.ticks() {
        if plot.radius_at(vs, t) < 1.0 {
            continue; // the centre ring collapses to the pole — skip the degenerate polygon
        }
        let mut poly: Vec<(f64, f64)> = (0..n).map(|i| plot.project(xs, i as f64, vs, t)).collect();
        if let Some(&p0) = poly.first() {
            poly.push(p0); // close the ring
        }
        out.push(prim::line(poly, grid.clone(), 1.0));
    }
    let centre = plot.center();
    for i in 0..n {
        let rim = crate::layout::geom::polar(centre, plot.radius(), plot.spoke_angle(xs, i as f64));
        out.push(prim::line(vec![centre, rim], grid.clone(), 1.0));
    }
}

/// The spoke (category) labels just outside the rim, and the radius tick labels up the
/// top spoke (offset right so they clear it).
pub fn labels(plot: &Plot, chart: &Chart, out: &mut Vec<PlacedNode>) {
    let Scale::Band { n } = chart.x.scale else {
        return;
    };
    let xs = &chart.x.scale;
    let vs = &chart.values[0].scale;
    let (cx, cy) = plot.center();
    let muted = ResolvedValue::live("muted");
    let cl = chart.clearance;
    let radius = plot.radius();
    for i in 0..n {
        let label = chart
            .x
            .labels
            .get(i)
            .cloned()
            .unwrap_or_else(|| (i + 1).to_string());
        // The label stands `clearance` off the **rim**, edge to edge — so a
        // long name to the side clears by the same daylight as a short one on
        // top [SPEC 14.6]. Its centre therefore sits one clearance plus its own
        // reach along the spoke: the box's support in that direction, which for
        // an axis-aligned rectangle is |dx|·w/2 + |dy|·h/2.
        let (rx, ry) = crate::layout::geom::polar((cx, cy), radius, plot.spoke_angle(xs, i as f64));
        let (dx, dy) = if radius > 0.0 {
            ((rx - cx) / radius, (ry - cy) / radius)
        } else {
            (0.0, -1.0)
        };
        let w = text::width(&label, TEXT, chart.font_kind);
        let h = prim::text_height(&label, TEXT_SIZE);
        let reach = cl + dx.abs() * w / 2.0 + dy.abs() * h / 2.0;
        out.push(text::centered(
            &label,
            rx + dx * reach,
            ry + dy * reach,
            TEXT,
            Some(muted.clone()),
            chart.font_kind,
        ));
    }
    for &t in vs.ticks() {
        let r = plot.radius_at(vs, t);
        if r < 1.0 {
            continue; // skip the centre tick — it would pile on the pole
        }
        let label = scale::label(
            &chart.values[0].scale,
            t,
            chart.values[0].fmt,
            &chart.values[0].unit,
        );
        out.push(text::left(
            &label,
            cx + cl,
            cy - r,
            TEXT,
            Some(muted.clone()),
            chart.font_kind,
        ));
    }
}
