use crate::jellyfin::MediaItem;

#[derive(Debug, Clone)]
pub struct BrowserState {
    pub parent_id: Option<String>,
    pub parent_kind: Option<String>,
    pub title: String,
    pub items: Vec<MediaItem>,
    pub selected: usize,
}

impl BrowserState {
    pub fn new(
        parent_id: Option<String>,
        parent_kind: Option<String>,
        title: String,
        items: Vec<MediaItem>,
    ) -> Self {
        Self::with_selection(parent_id, parent_kind, title, items, 0)
    }

    pub fn with_selection(
        parent_id: Option<String>,
        parent_kind: Option<String>,
        title: String,
        items: Vec<MediaItem>,
        selected: usize,
    ) -> Self {
        let selected = if items.is_empty() {
            0
        } else {
            selected.min(items.len().saturating_sub(1))
        };

        Self {
            parent_id,
            parent_kind,
            title,
            items,
            selected,
        }
    }

    pub fn is_season_view(&self) -> bool {
        self.parent_kind.as_deref() == Some("Season")
    }

    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = (self.selected + 1).min(self.items.len() - 1);
    }

    pub fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn selected_item(&self) -> Option<&MediaItem> {
        self.items.get(self.selected)
    }
}

/// Stack-backed navigator. Invariant: `stack` is never empty.
#[derive(Debug)]
pub struct Navigator {
    stack: Vec<BrowserState>,
}

impl Navigator {
    pub fn new(root: BrowserState) -> Self {
        Self { stack: vec![root] }
    }

    pub fn current(&self) -> &BrowserState {
        self.stack
            .last()
            .expect("navigator stack invariant: never empty")
    }

    pub fn current_mut(&mut self) -> &mut BrowserState {
        self.stack
            .last_mut()
            .expect("navigator stack invariant: never empty")
    }

    pub fn push(&mut self, state: BrowserState) {
        self.stack.push(state);
    }

    /// Pop the current state. Returns `false` if already at the root (stack
    /// unchanged), `true` otherwise.
    pub fn pop(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            true
        } else {
            false
        }
    }

    #[allow(dead_code)] // consumed in Task 4 by ui::render
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// The full navigation trail, root first, current last.
    #[allow(dead_code)] // consumed in Task 4 by ui::breadcrumb
    pub fn trail(&self) -> &[BrowserState] {
        &self.stack
    }

    /// Replace the current state in place (for reload).
    pub fn replace_current(&mut self, state: BrowserState) {
        let last_idx = self.stack.len() - 1;
        self.stack[last_idx] = state;
    }
}
