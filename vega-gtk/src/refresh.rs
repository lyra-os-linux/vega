//! Coalesce refresh triggers on the GTK main thread without queuing every click.
use std::{cell::Cell, ops::Deref, rc::Rc};

#[derive(Default)]
pub struct RefreshState {
    running: Cell<bool>,
    pending: Cell<bool>,
}

impl RefreshState {
    pub fn request(&self) -> bool {
        if self.running.replace(true) {
            self.pending.set(true);
            false
        } else {
            true
        }
    }

    pub fn finish(&self) -> bool {
        if self.pending.replace(false) {
            true
        } else {
            self.running.set(false);
            false
        }
    }
}

/// All writers of the updates card share the same refresh state.
#[derive(Clone)]
pub struct UpdatesCard {
    label: gtk::Label,
    pub refresh: Rc<RefreshState>,
}

impl UpdatesCard {
    pub fn new(label: gtk::Label) -> Self {
        Self {
            label,
            refresh: Rc::default(),
        }
    }
}

impl Deref for UpdatesCard {
    type Target = gtk::Label;
    fn deref(&self) -> &Self::Target {
        &self.label
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicks_and_timer_during_a_query_share_one_followup() {
        let state = RefreshState::default();
        assert!(state.request());
        for _ in 0..100 {
            assert!(!state.request());
        }
        assert!(state.finish());
        assert!(!state.finish());
        assert!(state.request());
        assert!(!state.finish());
    }

    #[test]
    fn requests_during_followup_are_not_lost() {
        let state = RefreshState::default();
        assert!(state.request());
        assert!(!state.request());
        assert!(state.finish());
        assert!(!state.request());
        assert!(state.finish());
        assert!(!state.finish());
    }
}
