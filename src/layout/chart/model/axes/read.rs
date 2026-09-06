//! The axis attribute readers [SPEC 14.4/16/20]: `range:`, `ticks:`, `step:`,
//! `side:`, `gridlines:`, `unit:`, `scale:`, and `format:`, plus the shared
//! domain-from-`range` resolution. `axes.rs` binds series to axes and builds the
//! scales; the parsing of each attribute lives here. [`Domain`] is what makes a
//! reader time-aware — one kind per axis, consulted wherever a value is placed
//! on one.

use super::super::*;

/// What an axis's values are written as [SPEC 14.3/14.4]: plain numbers, or
/// quoted ISO-8601 dates on a time axis. **One domain, one kind** — so every
/// reader that places a value on an axis consults it: the axis's own `range:`
/// and `ticks:`, a `|band|`'s span, a `|mark|`'s `at:`.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Domain {
    Number,
    Date,
}

/// The one mixed-domain reading [SPEC 21]: an axis is dated or numeric, and
/// everything measured on it follows — a series' x values, the axis's own
/// `range:` / `ticks:`, a `|band|`'s span, a `|mark|`'s `at:`.
pub(super) const MIXED_DOMAIN: &str =
    "an axis reads dates or numbers, never both — one domain, one kind";

impl Domain {
    /// One authored value on an axis of this kind, folded to the scale's own
    /// units (epoch seconds for a date). A value of the **other** kind is
    /// [`MIXED_DOMAIN`]; one that is neither falls to `otherwise` — the
    /// caller's own property wording, reachable on a numeric axis alone.
    pub(crate) fn value(
        self,
        v: &ResolvedValue,
        otherwise: &str,
        span: Span,
    ) -> Result<f64, Error> {
        match (self, v) {
            (Domain::Date, ResolvedValue::String(text)) => date_secs(text, span),
            (Domain::Number, v) if !matches!(v, ResolvedValue::String(_)) => {
                v.as_number().ok_or_else(|| Error::at(span, otherwise))
            }
            _ => Err(Error::at(span, MIXED_DOMAIN)),
        }
    }
}

/// An axis's `range:` [SPEC 14.4] — a two-item tuple, each end `auto` or a value
/// in the axis's `domain`.
pub(super) fn read_range(inst: &ResolvedInst, domain: Domain) -> Result<Option<(End, End)>, Error> {
    const ENDS: &str = "'range' takes two ends: 'a b', 'a auto', or 'auto b'";
    let Some(v) = inst.attrs.get("range") else {
        return Ok(None);
    };
    let ResolvedValue::Tuple(items) = v else {
        return Err(Error::at(inst.span, ENDS));
    };
    if items.len() != 2 {
        return Err(Error::at(inst.span, ENDS));
    }
    let end = |v: &ResolvedValue| match v {
        ResolvedValue::Ident(s) if s == "auto" => Ok(End::Auto),
        v => domain
            .value(v, "a 'range' end is a number or 'auto'", inst.span)
            .map(End::Num),
    };
    Ok(Some((end(&items[0])?, end(&items[1])?)))
}

/// An axis's explicit `ticks:` [SPEC 2/14.4] — a comma-list (or a lone value),
/// each item a value in the axis's `domain`.
pub(super) fn read_ticks(
    attrs: &AttrMap,
    domain: Domain,
    span: Span,
) -> Result<Option<Vec<f64>>, Error> {
    const TICKS: &str = "'ticks' takes comma-separated numbers — 'ticks: 0, 50, 100'";
    let Some(v) = attrs.get("ticks") else {
        return Ok(None);
    };
    let items = match v {
        ResolvedValue::List(items) => items.as_slice(),
        one => std::slice::from_ref(one),
    };
    items
        .iter()
        .map(|it| domain.value(it, TICKS, span))
        .collect::<Result<Vec<f64>, Error>>()
        .map(Some)
}

/// A calendar `step:` [SPEC 14.4] — a unit ident with an optional count
/// (`step: month`, `step: 2 week`); a plain number points at the calendar form.
pub(super) fn read_cal_step(inst: &ResolvedInst) -> Result<Option<(scale::CalUnit, u32)>, Error> {
    const CAL: &str = "a time axis steps by calendar — 'step: month', 'step: 2 week'";
    let unit = |s: &str| -> Option<scale::CalUnit> {
        Some(match s {
            "minute" => scale::CalUnit::Minute,
            "hour" => scale::CalUnit::Hour,
            "day" => scale::CalUnit::Day,
            "week" => scale::CalUnit::Week,
            "month" => scale::CalUnit::Month,
            "year" => scale::CalUnit::Year,
            _ => return None,
        })
    };
    match inst.attrs.get("step") {
        None => Ok(None),
        Some(ResolvedValue::Ident(s)) => match unit(s) {
            Some(u) => Ok(Some((u, 1))),
            None => Err(Error::at(inst.span, CAL)),
        },
        Some(ResolvedValue::Tuple(items)) => match items.as_slice() {
            [ResolvedValue::Number(n), ResolvedValue::Ident(s)]
                if n.fract() == 0.0 && (1.0..=1000.0).contains(n) =>
            {
                match unit(s) {
                    Some(u) => Ok(Some((u, *n as u32))),
                    None => Err(Error::at(inst.span, CAL)),
                }
            }
            _ => Err(Error::at(inst.span, CAL)),
        },
        Some(_) => Err(Error::at(inst.span, CAL)),
    }
}

/// A quoted date literal to epoch seconds, with the SPEC 21 message.
fn date_secs(text: &str, span: Span) -> Result<f64, Error> {
    date::parse(text).ok_or_else(|| {
        Error::at(
            span,
            format!("'{text}' is not a date — ISO-8601: '2026-01-31', optionally 'T09:30' and 'Z'"),
        )
    })
}

/// The domain from a value list, an explicit `range:` window, and the empty-data
/// fallback: `(min, max, reversed)`. A `range:` end of `auto` takes the data bound;
/// a high→low range reverses. Shared by the numeric x, time, and value scales (the
/// value scale supplies its own bars-include-zero `None` branch instead).
pub(super) fn resolve_domain(
    xs: &[f64],
    range: Option<&(End, End)>,
    empty: (f64, f64),
) -> (f64, f64, bool) {
    let data_min = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let data_max = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let (dmin, dmax) = if xs.is_empty() {
        empty
    } else {
        (data_min, data_max)
    };
    match range {
        Some((a, b)) => {
            let lo = end(a, dmin);
            let hi = end(b, dmax);
            (lo.min(hi), lo.max(hi), lo > hi)
        }
        None => (dmin, dmax, false),
    }
}

/// A numeric consumer's `format:` [SPEC 17]: its own (a date preset authored
/// here errors — it reads a time axis), else the chart's numeric reading.
pub(crate) fn numeric_fmt(inst: &ResolvedInst, chart_fmt: Format) -> Result<Format, Error> {
    let f = format::read_or(&inst.attrs, format::numeric(chart_fmt), inst.span)?;
    if inst.attrs.get("format").is_some() {
        format::reject_date(f, inst.span)?;
    }
    Ok(format::numeric(f))
}

/// An axis's `scale:` kind [SPEC 14.4]: `linear` (default), `log`, or `time`.
#[derive(PartialEq, Clone, Copy)]
pub(super) enum ScaleKind {
    Linear,
    Log,
    Time,
}

pub(super) fn read_scale_kind(inst: &ResolvedInst) -> Result<ScaleKind, Error> {
    match inst.attrs.get("scale") {
        None => Ok(ScaleKind::Linear),
        Some(ResolvedValue::Ident(s)) if s == "linear" => Ok(ScaleKind::Linear),
        Some(ResolvedValue::Ident(s)) if s == "log" => Ok(ScaleKind::Log),
        Some(ResolvedValue::Ident(s)) if s == "time" => Ok(ScaleKind::Time),
        _ => Err(Error::at(inst.span, "'scale' is linear, log, or time")),
    }
}

pub(super) fn read_log(inst: &ResolvedInst) -> Result<bool, Error> {
    Ok(read_scale_kind(inst)? == ScaleKind::Log)
}

pub(crate) fn read_side(inst: &ResolvedInst) -> Result<Option<Side>, Error> {
    match inst.attrs.get("side") {
        None => Ok(None),
        Some(ResolvedValue::Ident(s)) => match s.as_str() {
            "bottom" => Ok(Some(Side::Bottom)),
            "top" => Ok(Some(Side::Top)),
            "left" => Ok(Some(Side::Left)),
            "right" => Ok(Some(Side::Right)),
            _ => Err(Error::at(
                inst.span,
                "'side' is bottom, top, left, or right",
            )),
        },
        _ => Err(Error::at(
            inst.span,
            "'side' is bottom, top, left, or right",
        )),
    }
}

pub(super) fn read_grid(inst: &ResolvedInst) -> Result<Grid, Error> {
    match inst.attrs.get("gridlines") {
        None => Ok(Grid::Default),
        Some(ResolvedValue::Ident(s)) if s == "none" => Ok(Grid::Off),
        Some(v) => Ok(Grid::Color(v.clone())),
    }
}

pub(super) fn end(e: &End, auto: f64) -> f64 {
    match e {
        End::Num(n) => *n,
        End::Auto => auto,
    }
}

pub(super) fn read_unit(inst: &ResolvedInst) -> Result<Option<String>, Error> {
    match inst.attrs.get("unit") {
        None => Ok(None),
        Some(ResolvedValue::String(s)) => Ok(Some(s.clone())),
        _ => Err(Error::at(inst.span, "'unit' is a quoted string")),
    }
}
