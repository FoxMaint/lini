//! `|band|` and `|mark|` annotations, bound to an axis and placed by value.

use super::*;

/// The domain a value measured on `axis` is written in [SPEC 14.4]: the x axis's
/// own kind, or numbers on a value axis — which is never dated, since
/// [`axis_spec`] rejects `scale: time` there.
fn domain_of(axis: &AxisRef, x: Domain) -> Domain {
    match axis {
        AxisRef::X => x,
        AxisRef::Value(_) => Domain::Number,
    }
}

/// Parse a `|band|` [SPEC 14.5]: its bound axis (default the x/domain axis), its
/// `span`, label, and fill (a real fill shades; `none` / unset makes it a divider).
pub(super) fn read_band(
    inst: &ResolvedInst,
    x_id: Option<&str>,
    specs: &[AxisSpec],
    x: Domain,
) -> Result<Band, Error> {
    let axis = match axis_id(inst) {
        Some(id) => lookup_axis(id, x_id, specs, inst.span)?,
        None => AxisRef::X,
    };
    let fill = real_color(inst.attrs.get("fill"));
    let tick = fill.clone().unwrap_or_else(muted);
    Ok(Band {
        span: read_span(inst, domain_of(&axis, x))?,
        axis,
        label: label_of(inst),
        fill,
        tick,
    })
}

/// Parse a `|mark|` [SPEC 14.5]: a required bound axis, its `at` placement, the
/// label, whether a point shows its dot, and the accent (`stroke` / `fill`, else muted).
pub(super) fn read_mark(
    inst: &ResolvedInst,
    x_id: Option<&str>,
    specs: &[AxisSpec],
    chart_tip: Tooltip,
    x: Domain,
) -> Result<Mark, Error> {
    let axis = match axis_id(inst) {
        Some(id) => lookup_axis(id, x_id, specs, inst.span)?,
        None => return Err(Error::at(inst.span, "a '|mark|' needs 'axis:' to place it")),
    };
    let color = real_color(inst.attrs.get("stroke"))
        .or_else(|| real_color(inst.attrs.get("fill")))
        .unwrap_or_else(muted);
    Ok(Mark {
        at: read_at(inst, domain_of(&axis, x), x)?,
        axis,
        label: label_of(inst),
        marker: chart_marker(inst)?,
        color,
        stroke_style: inst.attrs.get("stroke-style").cloned(),
        tooltip: super::tooltip::read_or(&inst.attrs, chart_tip)?,
    })
}

/// A `|band|`'s `range: a b` — its data range on the bound axis [SPEC 14.5],
/// the same interval shape (and the same `domain`) an `|axis|` reads, so a band
/// on a time axis is written in dates.
fn read_span(inst: &ResolvedInst, domain: Domain) -> Result<(f64, f64), Error> {
    const ENDS: &str = "a band's 'range' ends are numbers";
    match inst.attrs.get("range") {
        Some(ResolvedValue::Tuple(items)) if items.len() == 2 => Ok((
            domain.value(&items[0], ENDS, inst.span)?,
            domain.value(&items[1], ENDS, inst.span)?,
        )),
        _ if inst.attrs.get("span").is_some() => Err(Error::at(
            inst.span,
            "a band's extent is 'range: a b' — 'span' places a grid child",
        )),
        _ => Err(Error::at(inst.span, "a '|band|' needs 'range: a b'")),
    }
}

/// A `|mark|`'s `at:` [SPEC 14.5] — one value (a reference line) on its `bound`
/// axis, or two (a point): the first on the domain axis (`x`), the second the
/// value on the bound one, which is always a number.
pub(super) fn read_at(inst: &ResolvedInst, bound: Domain, x: Domain) -> Result<MarkAt, Error> {
    const AT: &str = "'at' takes one value (a line) or two (a point)";
    match inst.attrs.get("at") {
        Some(ResolvedValue::Tuple(items)) if items.len() == 2 => Ok(MarkAt::Point(
            x.value(&items[0], AT, inst.span)?,
            Domain::Number.value(&items[1], AT, inst.span)?,
        )),
        Some(v) if !matches!(v, ResolvedValue::Tuple(_) | ResolvedValue::List(_)) => {
            bound.value(v, AT, inst.span).map(MarkAt::Line)
        }
        _ => Err(Error::at(inst.span, AT)),
    }
}
