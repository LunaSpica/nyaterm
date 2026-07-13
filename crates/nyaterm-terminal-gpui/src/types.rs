//! Shared terminal paint types.

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
