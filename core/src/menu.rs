//! The pause menu's navigation model: a tree of entries plus the path/cursor walked by d-pad
//! and touch. Pure, so the walking rules (wrap, push, pop, activate) are enforced by tests
//! instead of by replaying them on a console.
//!
//! The app supplies its own entry ids (`Id`); this crate knows nothing about what an entry
//! *does*. Activation returns the id - deciding is the app's business. Submenus push onto the
//! path; Back pops one level, and reports being at the root so the caller can close the menu.

/// One node of the menu tree: either a leaf the app must act on, or a submenu of further nodes.
/// `Id: 'static` because the submenu slice is `'static` (the trees are built as consts).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuNode<Id: 'static> {
    Leaf(Id),
    Submenu(Id, &'static [MenuNode<Id>]),
}

/// Where the walker currently is: the chain of submenu indices from the root, and the cursor
/// row on the current page. `path` is empty at the root page. Not generic - the ids live in the
/// tree, not in the walker, so one state type walks any tree.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MenuState {
    path: Vec<usize>,
    cursor: usize,
}

/// What activating the cursor row means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activated<Id> {
    /// A leaf: the app should run this id and (by convention) close the menu.
    Leaf(Id),
    /// A submenu: already pushed; repaint.
    Pushed,
    /// The page has no rows; nothing to activate.
    Nothing,
}

impl MenuState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Index chain of submenus taken from the root. Empty means "on the root page".
    pub fn path(&self) -> &[usize] {
        &self.path
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn depth(&self) -> usize {
        self.path.len()
    }

    /// The entries of the page the walker is on.
    pub fn page<'a, Id: Copy + 'static>(&self, root: &'a [MenuNode<Id>]) -> &'a [MenuNode<Id>] {
        let mut page = root;
        for &step in &self.path {
            match page.get(step) {
                Some(MenuNode::Submenu(_, children)) => page = children,
                _ => return &[],
            }
        }
        page
    }

    /// Ids from the root to the current page, for the breadcrumb line.
    pub fn breadcrumb<Id: Copy + 'static>(&self, root: &[MenuNode<Id>]) -> Vec<Id> {
        let mut ids = Vec::with_capacity(self.path.len());
        let mut page = root;
        for &step in &self.path {
            match page.get(step) {
                Some(MenuNode::Submenu(id, children)) => {
                    ids.push(*id);
                    page = children;
                }
                _ => break,
            }
        }
        ids
    }

    /// Moves the cursor one row, wrapping at both ends. A no-op on an empty page.
    pub fn move_cursor<Id: Copy + 'static>(&mut self, root: &[MenuNode<Id>], down: bool) {
        let len = self.page(root).len();
        if len == 0 {
            self.cursor = 0;
            return;
        }
        self.cursor = if down {
            (self.cursor + 1) % len
        } else if self.cursor == 0 {
            len - 1
        } else {
            self.cursor - 1
        };
    }

    /// Points the cursor at a row the caller computed (a tap, usually), clamped into range.
    pub fn set_cursor<Id: Copy + 'static>(&mut self, root: &[MenuNode<Id>], index: usize) {
        let len = self.page(root).len();
        self.cursor = if len == 0 { 0 } else { index.min(len - 1) };
    }

    /// Activates the cursor row: a leaf reports its id; a submenu is pushed and reports as
    /// such. The cursor of a freshly entered page starts at the first row.
    pub fn activate<Id: Copy + 'static>(&mut self, root: &[MenuNode<Id>]) -> Activated<Id> {
        match self.page(root).get(self.cursor) {
            Some(MenuNode::Leaf(id)) => Activated::Leaf(*id),
            Some(MenuNode::Submenu(_, _)) => {
                self.path.push(self.cursor);
                self.cursor = 0;
                Activated::Pushed
            }
            None => Activated::Nothing,
        }
    }

    /// Pops one submenu level. Returns false at the root - the caller's cue to close the menu.
    pub fn back(&mut self) -> bool {
        match self.path.pop() {
            Some(parent_index) => {
                // Return to the row that opened this page, so Back-Confirm retraces forward.
                self.cursor = parent_index;
                true
            }
            None => false,
        }
    }

    /// Resets to the root page, first row - what "open the menu" means.
    pub fn reset(&mut self) {
        self.path.clear();
        self.cursor = 0;
    }

    /// The id of the row the cursor is on, if any.
    pub fn current_id<Id: Copy + 'static>(&self, root: &[MenuNode<Id>]) -> Option<Id> {
        match self.page(root).get(self.cursor) {
            Some(MenuNode::Leaf(id)) => Some(*id),
            Some(MenuNode::Submenu(id, _)) => Some(*id),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Activated, MenuNode, MenuState};

    const SUB_A: &[MenuNode<u8>] = &[MenuNode::Leaf(10), MenuNode::Leaf(11)];
    const SUB_DEEP: &[MenuNode<u8>] = &[MenuNode::Leaf(20)];
    const SUB_B: &[MenuNode<u8>] = &[
        MenuNode::Submenu(12, SUB_DEEP),
        MenuNode::Leaf(13),
        MenuNode::Leaf(14),
    ];
    const ROOT: &[MenuNode<u8>] = &[
        MenuNode::Leaf(1),
        MenuNode::Submenu(2, SUB_A),
        MenuNode::Submenu(3, SUB_B),
    ];

    #[test]
    fn cursor_wraps_both_ways() {
        let mut state = MenuState::new();
        state.move_cursor(ROOT, false);
        assert_eq!(state.cursor(), 2, "up from row 0 wraps to the last row");
        state.move_cursor(ROOT, true);
        assert_eq!(state.cursor(), 0, "down from the last row wraps to row 0");
    }

    #[test]
    fn activate_leaf_reports_id_without_moving() {
        let mut state = MenuState::new();
        assert_eq!(state.current_id(ROOT), Some(1), "cursor starts on row 0");
        assert_eq!(state.activate(ROOT), Activated::Leaf(1));
        assert_eq!(state.path(), &[] as &[usize], "a leaf does not navigate");
        assert_eq!(state.cursor(), 0);
    }

    #[test]
    fn submenu_push_resets_cursor_and_back_returns_to_opener() {
        let mut state = MenuState::new();
        state.move_cursor(ROOT, true); // row 1: submenu A
        assert_eq!(state.activate(ROOT), Activated::Pushed);
        assert_eq!((state.depth(), state.cursor()), (1, 0));
        assert_eq!(state.page(ROOT).len(), 2);
        assert!(state.back());
        assert_eq!((state.depth(), state.cursor()), (0, 1), "Back lands on the opener");
    }

    #[test]
    fn deep_pages_nest_and_breadcrumb_traces_the_way_in() {
        let mut state = MenuState::new();
        state.move_cursor(ROOT, true);
        state.move_cursor(ROOT, true); // row 2: submenu B
        assert_eq!(state.activate(ROOT), Activated::Pushed);
        assert_eq!(state.activate(ROOT), Activated::Pushed, "row 0 of B is a submenu");
        assert_eq!(state.depth(), 2);
        assert_eq!(state.page(ROOT).len(), 1);
        assert_eq!(state.breadcrumb(ROOT), vec![3, 12]);
        assert_eq!(state.activate(ROOT), Activated::Leaf(20));
        assert!(state.back());
        assert!(state.back());
        assert!(!state.back(), "Back at the root means: close the menu");
    }

    #[test]
    fn empty_page_is_inert() {
        const EMPTY_ROOT: &[MenuNode<u8>] = &[];
        let mut state = MenuState::new();
        assert_eq!(state.activate(EMPTY_ROOT), Activated::Nothing);
        state.move_cursor(EMPTY_ROOT, true);
        assert_eq!(state.cursor(), 0);
        assert_eq!(state.current_id(EMPTY_ROOT), None);
        state.set_cursor(EMPTY_ROOT, 5);
        assert_eq!(state.cursor(), 0);
    }

    #[test]
    fn set_cursor_clamps_and_reset_goes_home() {
        let mut state = MenuState::new();
        state.set_cursor(ROOT, 99);
        assert_eq!(state.cursor(), 2);
        state.reset();
        assert_eq!((state.depth(), state.cursor()), (0, 0));
    }

    #[test]
    fn current_id_reads_leaf_and_submenu_rows() {
        let mut state = MenuState::new();
        assert_eq!(state.current_id(ROOT), Some(1));
        state.move_cursor(ROOT, true);
        assert_eq!(state.current_id(ROOT), Some(2));
    }
}
