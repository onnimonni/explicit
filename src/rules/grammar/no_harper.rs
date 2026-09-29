//! Stand-in for the Harper side of the checker in builds without the `harper` feature. The
//! checker only runs the `spellbook` engine there, so none of this is ever called.

use super::RawLints;
use crate::config::Config;
use crate::rules::words::Dialect;

pub(super) struct Harper {
    _never: (),
}

pub(super) struct Setup {
    pub harper: Harper,
    pub spell_md: bool,
    pub spell_code: bool,
    pub harper_md: bool,
    pub harper_code: bool,
}

impl Harper {
    pub(super) fn setup(_: &Config, _: Dialect, _: bool) -> Setup {
        unreachable!("the checker runs `spellbook` in builds without Harper")
    }
    pub(super) fn select(&mut self, _: bool) {
        unreachable!()
    }
    pub(super) fn rule_on(&self, _: &str) -> bool {
        unreachable!()
    }
    pub(super) fn contains(&self, _: &str) -> bool {
        unreachable!()
    }
    pub(super) fn contains_in_dialect(&self, _: &str) -> bool {
        unreachable!()
    }
    pub(super) fn lints(&mut self, _: &str, _: bool, _: bool) -> RawLints {
        unreachable!()
    }
    pub(super) fn suggestions(&self, _: &str) -> Vec<String> {
        unreachable!()
    }
}
