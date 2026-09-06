//! Axis-aligned rectangle — the one geometric primitive the router shares.

use crate::ast::Side;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl Rect {
    pub fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
        Rect { x0, y0, x1, y1 }
    }

    pub fn w(&self) -> f64 {
        self.x1 - self.x0
    }

    pub fn h(&self) -> f64 {
        self.y1 - self.y0
    }

    pub fn centre(&self) -> (f64, f64) {
        ((self.x0 + self.x1) / 2.0, (self.y0 + self.y1) / 2.0)
    }

    /// A side's span on **its own** ordinate axis — the `y` range of a vertical
    /// side, the `x` range of a horizontal one. The axis every port ordinate,
    /// window, and corner margin is measured along.
    pub fn side_span(&self, side: Side) -> (f64, f64) {
        match side {
            Side::Left | Side::Right => (self.y0, self.y1),
            Side::Top | Side::Bottom => (self.x0, self.x1),
        }
    }

    /// Grow by `d` on every side (the keep-out construction).
    pub fn inflate(&self, d: f64) -> Rect {
        Rect::new(self.x0 - d, self.y0 - d, self.x1 + d, self.y1 + d)
    }

    /// Whether `r` sits wholly within this rect — the containment test
    /// "a body's contents ride inside it" reads (ROUTING.md Vocabulary).
    pub fn holds(&self, r: Rect) -> bool {
        r.x0 >= self.x0 && r.y0 >= self.y0 && r.x1 <= self.x1 && r.y1 <= self.y1
    }

    /// The overlap with positive area, if any — touching edges don't count.
    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let r = Rect::new(
            self.x0.max(other.x0),
            self.y0.max(other.y0),
            self.x1.min(other.x1),
            self.y1.min(other.y1),
        );
        (r.w() > 0.0 && r.h() > 0.0).then_some(r)
    }
}

/// The lawful **corner margin** on a side of length `len` at clearance `c`
/// (ROUTING.md Contact): the full clearance, relaxing to half the side where
/// the side is too short to hold one at each end.
pub(crate) fn port_margin(len: f64, c: f64) -> f64 {
    c.min(len / 2.0)
}

/// The lawful **port window** on a side: its span minus a [`port_margin`] at
/// each end, collapsing to the side's centre point when the side is too short.
/// The one window every stage reads — a graph entry, a natural port, and the
/// law checker's scarcity excuse.
pub(crate) fn port_window(rect: Rect, side: Side, c: f64) -> (f64, f64) {
    let (lo, hi) = rect.side_span(side);
    let m = port_margin(hi - lo, c);
    (lo + m, hi - m)
}

/// The port window an end carries onto a side of its **landing body**
/// (ROUTING.md Vocabulary): the endpoint's own span there, clamped into that
/// side's lawful window — the field's row on the card's edge, never nearer a
/// corner than the law allows. Landing on itself an end carries its whole
/// side, so an unclimbed contact is exactly [`port_window`].
pub(crate) fn carried_window(body: Rect, carry: Rect, side: Side, c: f64) -> (f64, f64) {
    let (lo, hi) = port_window(body, side, c);
    let (s0, s1) = carry.side_span(side);
    (s0.clamp(lo, hi), s1.clamp(lo, hi))
}

/// Distance between two axis-aligned boxes (as `(x0, y0, x1, y1)`);
/// segments degenerate to boxes. The one clearance metric the law checker
/// and the natural tightening pass share — a wire one judges legal, the
/// other does too.
pub(crate) fn box_dist(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> f64 {
    let dx = (b.0 - a.2).max(a.0 - b.2).max(0.0);
    let dy = (b.1 - a.3).max(a.1 - b.3).max(0.0);
    (dx * dx + dy * dy).sqrt()
}

/// A two-point segment's bounding box.
pub(crate) fn seg_box(s: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    (
        s[0].0.min(s[1].0),
        s[0].1.min(s[1].1),
        s[0].0.max(s[1].0),
        s[0].1.max(s[1].1),
    )
}

/// A [`Rect`] as the tuple `box_dist` consumes.
pub(crate) fn rect_box(r: Rect) -> (f64, f64, f64, f64) {
    (r.x0, r.y0, r.x1, r.y1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extent_is_signed_span() {
        let r = Rect::new(-10.0, 5.0, 30.0, 25.0);
        assert_eq!(r.w(), 40.0);
        assert_eq!(r.h(), 20.0);
    }

    #[test]
    fn inflate_grows_every_side() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0).inflate(8.0);
        assert_eq!(r, Rect::new(-8.0, -8.0, 18.0, 18.0));
    }

    #[test]
    fn intersect_returns_the_overlap() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, -5.0, 20.0, 5.0);
        assert_eq!(a.intersect(&b), Some(Rect::new(5.0, 0.0, 10.0, 5.0)));
    }

    #[test]
    fn a_body_holds_only_what_lies_wholly_within_it() {
        let card = Rect::new(0.0, 0.0, 100.0, 60.0);
        assert!(card.holds(Rect::new(1.0, 1.0, 40.0, 20.0)));
        assert!(card.holds(card));
        assert!(!card.holds(Rect::new(-1.0, 1.0, 40.0, 20.0)));
    }

    #[test]
    fn a_carried_window_is_the_endpoints_row_clamped_into_the_side() {
        let card = Rect::new(0.0, 0.0, 100.0, 60.0);
        // Landing on itself, a body carries its whole side: the port window.
        assert_eq!(
            carried_window(card, card, Side::Right, 16.0),
            port_window(card, Side::Right, 16.0)
        );
        // A middle row lands within its own span.
        let row = Rect::new(1.0, 20.0, 99.0, 40.0);
        assert_eq!(carried_window(card, row, Side::Right, 16.0), (20.0, 40.0));
        // A row past the corner margin clamps onto it — never nearer a
        // corner than Law 2 allows.
        let top = Rect::new(1.0, 1.0, 99.0, 10.0);
        assert_eq!(carried_window(card, top, Side::Right, 16.0), (16.0, 16.0));
        // A horizontal side carries the endpoint's column instead.
        assert_eq!(carried_window(card, row, Side::Bottom, 16.0), (16.0, 84.0));
    }

    #[test]
    fn intersect_is_none_for_disjoint_and_touching() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert_eq!(a.intersect(&Rect::new(20.0, 0.0, 30.0, 10.0)), None);
        assert_eq!(a.intersect(&Rect::new(10.0, 0.0, 30.0, 10.0)), None);
    }
}
