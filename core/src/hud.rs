//! Performance-panel logic: parsing the peer's stats line and the ring buffers behind the
//! charts. Pure, like everything else in this crate - the pause menu and the HUD both live or
//! die by "did the numbers move", so the maths is testable on any PC.

use crate::hud::Grade::{Bad, Good, Warn};

/// How a displayed metric is judged. The grade picks the colour at the paint site; thresholds
/// live with the metric, here, so two widgets cannot disagree about what "bad" means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    Good,
    Warn,
    Bad,
    Neutral,
}

/// One peer-stats snapshot, parsed from the space-separated `key:value` line the peer publishes
/// (`peer.rs` stats format: `fps:60 kbps:18200 loss:0.2% rtt:38 src:... dec:3.4ms jit:4.1ms ...`).
/// Every field is optional: a token that is absent or unparsable is `None`, not 0, so the HUD
/// can tell "no data" from "measured zero".
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HudSample {
    pub kbps: Option<f32>,
    pub loss_pct: Option<f32>,
    pub rtt_ms: Option<f32>,
    pub jitter_ms: Option<f32>,
    pub decode_ms: Option<f32>,
    pub drop_per_sec: Option<f32>,
}

impl HudSample {
    /// Parses the tokens it knows and ignores the rest of the line. Tolerant by design: the
    /// line's format is the peer's business and grows new tokens; a consumer that breaks on an
    /// unknown token is a consumer that breaks on every peer improvement.
    pub fn parse(line: &str) -> Self {
        let mut sample = HudSample::default();
        for token in line.split_whitespace() {
            let Some((key, raw)) = token.split_once(':') else {
                continue;
            };
            let value = parse_metric(raw);
            match key {
                "kbps" => sample.kbps = value,
                "loss" => sample.loss_pct = value,
                "rtt" => sample.rtt_ms = value,
                "jit" => sample.jitter_ms = value,
                "dec" => sample.decode_ms = value,
                "drop" => sample.drop_per_sec = value,
                _ => {}
            }
        }
        sample
    }
}

/// `"<f32>"` with an optional trailing unit (`%`, `ms`, `/s`, `s`), or None. A trailing `-`
/// (the peer's "no value yet") parses as None.
fn parse_metric(raw: &str) -> Option<f32> {
    let raw = raw
        .trim_end_matches('%')
        .trim_end_matches("ms")
        .trim_end_matches("/s")
        .trim_end_matches('s');
    raw.parse::<f32>().ok().filter(|v| v.is_finite())
}

/// Grades for the metrics the HUD displays. Thresholds follow the ones the client already used
/// inline (fps colour bands, loss going red at 2 %) plus round numbers where none existed;
/// they are named here so the pause menu and the HUD cannot drift apart.
pub fn fps_grade(fps: f32) -> Grade {
    if fps >= 55.0 {
        Good
    } else if fps >= 30.0 {
        Warn
    } else {
        Bad
    }
}

pub fn loss_grade(loss_pct: f32) -> Grade {
    if loss_pct < 0.5 {
        Good
    } else if loss_pct < 2.0 {
        Warn
    } else {
        Bad
    }
}

pub fn rtt_grade(rtt_ms: f32) -> Grade {
    if rtt_ms < 60.0 {
        Good
    } else if rtt_ms < 120.0 {
        Warn
    } else {
        Bad
    }
}

pub fn jitter_grade(jitter_ms: f32) -> Grade {
    if jitter_ms < 8.0 {
        Good
    } else if jitter_ms < 20.0 {
        Warn
    } else {
        Bad
    }
}

pub fn decode_grade(decode_ms: f32) -> Grade {
    if decode_ms < 12.0 {
        Good
    } else if decode_ms < 25.0 {
        Warn
    } else {
        Bad
    }
}

/// Fixed-capacity ring of `f32` samples for a chart. Index 0 is the oldest sample still held;
/// `push` overwrites the oldest once full. No allocation after construction.
#[derive(Debug, Clone)]
pub struct RingF32 {
    data: Vec<f32>,
    capacity: usize,
    next: usize,
    len: usize,
}

impl RingF32 {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0.0; capacity],
            capacity,
            next: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, value: f32) {
        if self.capacity == 0 {
            return;
        }
        self.data[self.next] = value;
        self.next = (self.next + 1) % self.capacity;
        self.len = (self.len + 1).min(self.capacity);
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Oldest-held sample first: `at(0)` is the oldest, `at(len-1)` the newest.
    pub fn at(&self, index: usize) -> f32 {
        debug_assert!(index < self.len, "ring index out of range");
        // Modular, not saturating: when the write head has wrapped past the start, the oldest
        // sample sits *after* the head, and a saturating subtraction would read the newest
        // slice instead of the oldest.
        let start = (self.next + self.capacity - self.len) % self.capacity;
        self.data[(start + index) % self.capacity]
    }

    pub fn latest(&self) -> Option<f32> {
        (self.len > 0).then(|| self.at(self.len - 1))
    }

    /// The chart's vertical scale: the largest value held, or `floor_max` when everything is
    /// quieter than that - a chart that rescales down to noise amplifies the noise instead.
    pub fn upper_bound(&self, floor_max: f32) -> f32 {
        let max = (0..self.len).map(|i| self.at(i)).fold(0.0_f32, f32::max);
        max.max(floor_max)
    }

    pub fn clear(&mut self) {
        self.next = 0;
        self.len = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = "fps:60 kbps:18234 loss:0.2% rtt:38 src:18000 sub:2 qf:0 dec:3.4ms \
                        wait:1.2ms jit:4.1ms nof:0 err:0 reb:0 stall:0 rtp:18000 drop:0.0 pli:1/2 \
                        nack:3(0/s) rtx:4(0/s) lost:5(+1) rescue:0/s expire:0/s wfk:0 in:1 pr:0";

    #[test]
    fn parses_known_tokens_and_ignores_the_rest() {
        let s = HudSample::parse(LINE);
        assert_eq!(s.kbps, Some(18234.0));
        assert_eq!(s.loss_pct, Some(0.2));
        assert_eq!(s.rtt_ms, Some(38.0));
        assert_eq!(s.jitter_ms, Some(4.1));
        assert_eq!(s.decode_ms, Some(3.4));
        assert_eq!(s.drop_per_sec, Some(0.0));
    }

    #[test]
    fn missing_tokens_are_none_not_zero() {
        let s = HudSample::parse("fps:60");
        assert_eq!(s, HudSample::default());
        assert_eq!(s.kbps, None);
    }

    #[test]
    fn dash_and_garbage_parse_as_none() {
        assert_eq!(parse_metric("-"), None);
        assert_eq!(parse_metric("abc"), None);
        assert_eq!(parse_metric(""), None);
    }

    #[test]
    fn units_are_stripped() {
        assert_eq!(parse_metric("12.5%"), Some(12.5));
        assert_eq!(parse_metric("3.4ms"), Some(3.4));
        assert_eq!(parse_metric("0.0/s"), Some(0.0));
        assert_eq!(parse_metric("4s"), Some(4.0));
    }

    #[test]
    fn grades_match_the_documented_thresholds() {
        assert_eq!(fps_grade(60.0), Good);
        assert_eq!(fps_grade(45.0), Warn);
        assert_eq!(fps_grade(20.0), Bad);
        assert_eq!(loss_grade(0.1), Good);
        assert_eq!(loss_grade(1.0), Warn);
        assert_eq!(loss_grade(2.5), Bad);
        assert_eq!(rtt_grade(50.0), Good);
        assert_eq!(rtt_grade(90.0), Warn);
        assert_eq!(rtt_grade(150.0), Bad);
        assert_eq!(jitter_grade(5.0), Good);
        assert_eq!(jitter_grade(12.0), Warn);
        assert_eq!(jitter_grade(30.0), Bad);
        assert_eq!(decode_grade(10.0), Good);
        assert_eq!(decode_grade(20.0), Warn);
        assert_eq!(decode_grade(30.0), Bad);
    }

    #[test]
    fn ring_holds_then_overwrites_in_order() {
        let mut ring = RingF32::new(3);
        for v in [1.0, 2.0, 3.0] {
            ring.push(v);
        }
        assert_eq!(ring.len(), 3);
        assert_eq!((ring.at(0), ring.at(1), ring.at(2)), (1.0, 2.0, 3.0));
        ring.push(4.0);
        assert_eq!(ring.len(), 3, "capacity is a ceiling, not a growth plan");
        assert_eq!((ring.at(0), ring.at(1), ring.at(2)), (2.0, 3.0, 4.0));
        assert_eq!(ring.latest(), Some(4.0));
    }

    #[test]
    fn ring_upper_bound_respects_floor() {
        let mut ring = RingF32::new(4);
        for v in [1.0, 2.0] {
            ring.push(v);
        }
        assert_eq!(ring.upper_bound(60.0), 60.0, "a flat chart beats a noisy one");
        for v in [90.0, 2.0] {
            ring.push(v);
        }
        assert_eq!(ring.upper_bound(60.0), 90.0);
    }

    #[test]
    fn empty_ring_is_inert() {
        let mut ring = RingF32::new(0);
        ring.push(1.0);
        assert!(ring.is_empty());
        assert_eq!(ring.latest(), None);
        assert_eq!(ring.upper_bound(60.0), 60.0);
    }

    #[test]
    fn clear_forgets_history() {
        let mut ring = RingF32::new(2);
        ring.push(5.0);
        ring.clear();
        assert!(ring.is_empty());
        ring.push(7.0);
        assert_eq!(ring.at(0), 7.0);
    }

    #[test]
    fn neutral_grade_exists_for_valueless_rows() {
        // Rows like "session time" carry no judgement; paint sites map Neutral to dim text.
        assert!(HudSample::default().kbps.is_none());
    }
}
