//! Shared terminal paint types.

#[derive(Clone)]
pub(super) struct TerminalHighlightSpan {
    pub(super) text: String,
    pub(super) color: Option<u32>,
    pub(super) bg: Option<u32>,
    pub(super) keyword: bool,
    pub(super) underline: bool,
    pub(super) strikeout: bool,
    pub(super) bold: bool,
    pub(super) italic: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TerminalKeywordRange {
    pub(super) start_col: usize,
    pub(super) end_col: usize,
    pub(super) color: u32,
}
