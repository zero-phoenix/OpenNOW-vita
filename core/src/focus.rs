//! Focus navigation state for gamepad-driven menus.
//!
//! The app keeps one focus index per screen (`settings_focus`, `server_picker_focus`, ...), each
//! with hand-written clamp-and-wrap arithmetic next to it. This is that arithmetic, written once:
//! a list whose index moves with wrapping, clamps when the list shrinks, and answers sensibly
//! when it is empty. Screens adopt it one at a time; the egui side paints the focus and maps
//! buttons to `next`/`prev`/`select`.

/// A focus index over a fixed-length list of rows, with wrap-around movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusList {
    index: usize,
    len: usize,
}

impl FocusList {
    /// A list of `len` rows, focused on row 0 (or nothing, when `len` is 0).
    pub fn new(len: usize) -> Self {
        Self { index: 0, len }
    }

    /// Focused row. Only meaningful when `is_empty()` is false; 0 when empty.
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Replaces the row count, clamping the focus into what is left. An empty list has no focus.
    pub fn set_len(&mut self, len: usize) {
        self.len = len;
        if len == 0 {
            self.index = 0;
        } else {
            self.index = self.index.min(len - 1);
        }
    }

    /// Points the focus at a row the caller computed (e.g. from a tap), clamped into range.
    pub fn set_index(&mut self, index: usize) {
        if self.len > 0 {
            self.index = index.min(self.len - 1);
        }
    }

    /// Jumps back to the first row (what most screens do when they open or reload).
    pub fn reset(&mut self) {
        self.index = 0;
    }

    /// Moves down, wrapping from the last row to the first. A no-op when empty.
    pub fn next(&mut self) {
        if self.len > 0 {
            self.index = (self.index + 1) % self.len;
        }
    }

    /// Moves up, wrapping from the first row to the last. A no-op when empty.
    pub fn prev(&mut self) {
        if self.len > 0 {
            self.index = if self.index == 0 {
                self.len - 1
            } else {
                self.index - 1
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FocusList;

    #[test]
    fn empty_list_never_moves_and_has_no_focus() {
        let mut list = FocusList::new(0);
        assert!(list.is_empty());
        list.next();
        list.prev();
        list.set_index(3);
        list.set_len(0);
        assert_eq!((list.index(), list.len()), (0, 0));
    }

    #[test]
    fn down_wraps_from_last_row_to_first() {
        let mut list = FocusList::new(3);
        list.set_index(2);
        list.next();
        assert_eq!(list.index(), 0);
    }

    #[test]
    fn up_wraps_from_first_row_to_last() {
        let mut list = FocusList::new(3);
        list.prev();
        assert_eq!(list.index(), 2);
    }

    #[test]
    fn shrinking_clamps_focus_into_range() {
        let mut list = FocusList::new(5);
        list.set_index(4);
        list.set_len(3);
        assert_eq!(list.index(), 2);
    }

    #[test]
    fn set_index_clamps_and_ignores_empty() {
        let mut list = FocusList::new(2);
        list.set_index(99);
        assert_eq!(list.index(), 1);
        list.set_len(0);
        list.set_index(1);
        assert_eq!(list.index(), 0);
    }

    #[test]
    fn single_row_is_stable_in_both_directions() {
        let mut list = FocusList::new(1);
        list.next();
        list.prev();
        assert_eq!(list.index(), 0);
    }

    #[test]
    fn full_walk_visits_every_row_once() {
        let mut list = FocusList::new(4);
        let mut visited = Vec::new();
        for _ in 0..4 {
            visited.push(list.index());
            list.next();
        }
        assert_eq!(visited, [0, 1, 2, 3]);
        assert_eq!(list.index(), 0, "wraps back to the start after a full lap");
    }

    #[test]
    fn reset_returns_to_first_row() {
        let mut list = FocusList::new(3);
        list.set_index(2);
        list.reset();
        assert_eq!(list.index(), 0);
    }
}
