//! A focusable single-line text field.
//!
//! The app previously drew "inputs" as label divs: a `div` that took focus,
//! swallowed key events, and printed the draft string. That has no caret, no
//! selection, no pointer positioning and no IME, so it reads as broken the
//! moment anyone tries to edit rather than append.
//!
//! This is a real GPUI widget instead — an [`Entity`] that owns its buffer and
//! focus, implements [`EntityInputHandler`] so the platform routes composition
//! and clipboard through it, and paints itself with a custom [`Element`] that
//! shapes the line once and reuses that shaping for hit-testing, the selection
//! quads and the caret. Owners react to edits by subscribing to
//! [`TextFieldEvent`] rather than polling the string.

use std::ops::Range;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    App, Bounds, ClickEvent, Context, CursorStyle, Element, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, EventEmitter, FocusHandle, Focusable, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    Point, Render, SharedString, Style, TextRun, UTF16Selection, Window, WrappedLine, div, fill,
    point, prelude::*, px, relative, rgb, size,
};
use nyaterm_core::{CursorMotion, TextEdit};

use crate::theme::ThemePalette;

/// Half of the caret's on/off period.
const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(530);

/// What a [`TextField`] tells its owner.
///
/// Deliberately only the edit: Enter, Escape, Tab and the arrows are dialog and
/// list concerns, so the field leaves them unconsumed for the owner's own key
/// handler to see rather than guessing what they should mean here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextFieldEvent {
    /// The buffer changed. Carries the new content so the owner does not have
    /// to read back through the entity.
    Changed(String),
}

/// A single-line editable field.
pub struct TextField {
    edit: TextEdit,
    focus: FocusHandle,
    placeholder: SharedString,
    /// Render every character as a bullet. The buffer itself stays in the
    /// clear — masking is presentation, and the owner still needs the secret.
    masked: bool,
    /// In-flight IME composition, as a range into the buffer.
    marked: Option<Range<usize>>,
    /// Wrap at the field's width and accept newlines, for a description box.
    multi_line: bool,
    /// Horizontal scroll, so a caret past the right edge stays visible. Only a
    /// single-line field scrolls sideways; a wrapped one grows downward instead.
    scroll_x: Pixels,
    /// Vertical scroll for the wrapped case.
    scroll_y: Pixels,
    caret_visible: bool,
    /// Focus as of the last render, so owners can style their own chrome with
    /// only an `&App` — focus itself is a window-scoped question.
    focused: bool,
    blink: Option<gpui::Task<()>>,
    selecting: bool,
}

impl TextField {
    pub fn new(cx: &mut Context<Self>, content: impl Into<String>) -> Self {
        Self {
            edit: TextEdit::new(content),
            focus: cx.focus_handle(),
            placeholder: SharedString::default(),
            masked: false,
            marked: None,
            multi_line: false,
            scroll_x: px(0.),
            scroll_y: px(0.),
            caret_visible: true,
            focused: false,
            blink: None,
            selecting: false,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Wrap at the field's width, and let Enter insert a newline.
    pub fn multi_line(mut self, multi_line: bool) -> Self {
        self.multi_line = multi_line;
        self
    }

    pub fn content(&self) -> &str {
        self.edit.content()
    }

    pub fn is_empty(&self) -> bool {
        self.edit.is_empty()
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    pub fn has_focus(&self) -> bool {
        self.focused
    }

    /// Replace the buffer from the owner's state without emitting a change.
    pub fn set_content(&mut self, content: impl Into<String>, cx: &mut Context<Self>) {
        let content = content.into();
        if self.edit.content() == content {
            return;
        }
        self.edit.set_content(content);
        self.marked = None;
        cx.notify();
    }

    /// Put the caret at the end and select everything, the usual "focus for
    /// replacement" behaviour when a dialog opens.
    pub fn select_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.edit.select_all();
        self.restart_blink(window, cx);
        cx.notify();
    }

    fn emit_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.marked = None;
        self.restart_blink(window, cx);
        cx.emit(TextFieldEvent::Changed(self.edit.content().to_string()));
        cx.notify();
    }

    /// Keep the caret solid for a moment after every edit, so typing does not
    /// flicker, then resume blinking.
    fn restart_blink(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.caret_visible = true;
        self.blink = Some(cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(CARET_BLINK_INTERVAL).await;
                // The task ends with focus, so an unfocused field costs nothing.
                let Ok(still_focused) = this.update_in(cx, |this, window, cx| {
                    let focused = this.focus.is_focused(window);
                    if focused {
                        this.caret_visible = !this.caret_visible;
                        cx.notify();
                    }
                    focused
                }) else {
                    return;
                };
                if !still_focused {
                    return;
                }
            }
        }));
    }

    /// Returns whether the key was an edit, so the caller only stops
    /// propagation for keys this field actually claimed.
    fn handle_key(
        &mut self,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let keystroke = &event.keystroke;
        let shift = keystroke.modifiers.shift;
        let word = keystroke.modifiers.control || keystroke.modifiers.alt;
        let accel = keystroke.modifiers.platform || keystroke.modifiers.control;

        match keystroke.key.as_str() {
            "a" if accel => {
                self.edit.select_all();
                self.restart_blink(window, cx);
                cx.notify();
                return true;
            }
            "c" | "x" if accel => {
                let selection = self.edit.selection();
                if !selection.is_empty() && !self.masked {
                    let text = self.edit.content()[selection.clone()].to_string();
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                    if keystroke.key == "x" {
                        self.edit.replace(selection, "");
                        self.emit_changed(window, cx);
                    }
                }
                return true;
            }
            "v" if accel => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    let text = prepare_pasted_text(&text, self.multi_line);
                    self.edit.insert(&text);
                    self.emit_changed(window, cx);
                }
                return true;
            }
            "left" => self
                .edit
                .move_cursor(motion(CursorMotion::Left, word), shift),
            "right" => self
                .edit
                .move_cursor(motion(CursorMotion::Right, word), shift),
            "home" => self.edit.move_cursor(CursorMotion::Start, shift),
            "end" => self.edit.move_cursor(CursorMotion::End, shift),
            "backspace" => {
                let changed = if word {
                    self.edit.delete_word_backward()
                } else {
                    self.edit.delete_backward()
                };
                if changed {
                    self.emit_changed(window, cx);
                }
                return true;
            }
            "delete" => {
                if self.edit.delete_forward() {
                    self.emit_changed(window, cx);
                }
                return true;
            }
            // A wrapped field owns the keys that move within it; a single-line
            // one leaves them all to the dialog.
            "enter" if self.multi_line && !accel => {
                self.edit.insert("\n");
                self.emit_changed(window, cx);
                return true;
            }
            "up" | "down" if self.multi_line => {
                self.move_cursor_by_line(keystroke.key == "down", shift);
                self.restart_blink(window, cx);
                cx.notify();
                return true;
            }
            // Navigation and dialog keys belong to the owner. Some of them do
            // carry a `key_char` (Tab produces a tab character), so they have to
            // be named, or they would be typed into the buffer instead.
            "tab" | "enter" | "escape" | "up" | "down" | "pageup" | "pagedown" => return false,
            _ => {
                // Everything else is text, unless a modifier claims it or the
                // key produces none.
                if accel || keystroke.modifiers.function {
                    return false;
                }
                // Space arrives with no `key_char` on some platforms, which is
                // why the terminal's key encoder names it too.
                let space = (keystroke.key == "space").then_some(" ");
                let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .or(space)
                else {
                    return false;
                };
                self.edit.insert(input);
                self.emit_changed(window, cx);
                return true;
            }
        }

        self.restart_blink(window, cx);
        cx.notify();
        true
    }
}

fn motion(base: CursorMotion, word: bool) -> CursorMotion {
    match (base, word) {
        (CursorMotion::Left, true) => CursorMotion::WordLeft,
        (CursorMotion::Right, true) => CursorMotion::WordRight,
        (base, _) => base,
    }
}

fn prepare_pasted_text(text: &str, multi_line: bool) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    if multi_line {
        normalized
    } else {
        normalized.replace('\n', " ")
    }
}

impl TextField {
    /// Move the caret one hard line up or down, keeping the column.
    ///
    /// Hard lines rather than visual rows: the wrapped layout only exists during
    /// paint, and for a description box the two coincide often enough that
    /// reaching for it would cost more than it buys.
    fn move_cursor_by_line(&mut self, down: bool, extend: bool) {
        let content = self.edit.content().to_string();
        let cursor = self.edit.cursor();
        let line_start = content[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let column = cursor - line_start;

        let target_start = if down {
            match content[cursor..].find('\n') {
                Some(offset) => cursor + offset + 1,
                None => {
                    self.edit.move_cursor(CursorMotion::End, extend);
                    return;
                }
            }
        } else {
            if line_start == 0 {
                self.edit.move_cursor(CursorMotion::Start, extend);
                return;
            }
            content[..line_start - 1]
                .rfind('\n')
                .map(|i| i + 1)
                .unwrap_or(0)
        };

        let target_end = content[target_start..]
            .find('\n')
            .map(|offset| target_start + offset)
            .unwrap_or(content.len());
        let target = (target_start + column).min(target_end);

        if extend {
            let anchor = if self.edit.has_selection() {
                let selection = self.edit.selection();
                if self.edit.selection_is_reversed() {
                    selection.end
                } else {
                    selection.start
                }
            } else {
                cursor
            };
            self.edit
                .set_selection(anchor.min(target)..anchor.max(target), target < anchor);
        } else {
            self.edit.set_cursor(target);
        }
    }
}

impl EventEmitter<TextFieldEvent> for TextField {}

impl Focusable for TextField {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// The text the field actually paints, which is bullets when masked.
fn display_text(edit: &TextEdit, masked: bool) -> String {
    if masked {
        "•".repeat(edit.content().chars().count())
    } else {
        edit.content().to_string()
    }
}

/// Map a buffer offset onto an offset into the displayed text.
///
/// They differ only while masked, where one character can be several bytes in
/// the buffer but always three in the bullet string.
fn display_offset(edit: &TextEdit, masked: bool, offset: usize) -> usize {
    if !masked {
        return offset;
    }
    edit.content()[..offset.min(edit.content().len())]
        .chars()
        .count()
        * '•'.len_utf8()
}

fn buffer_offset(edit: &TextEdit, masked: bool, display: usize) -> usize {
    if !masked {
        return edit.floor_boundary(display);
    }
    let chars = display / '•'.len_utf8();
    edit.content()
        .char_indices()
        .nth(chars)
        .map(|(index, _)| index)
        .unwrap_or(edit.content().len())
}

impl Render for TextField {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window);
        self.focused = focused;
        div()
            .id("text-field")
            .track_focus(&self.focus)
            .size_full()
            .cursor(CursorStyle::IBeam)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if this.handle_key(event, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus);
                    this.selecting = true;
                    this.restart_blink(window, cx);
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _, _| {
                    this.selecting = false;
                }),
            )
            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                if event.click_count() >= 2 {
                    let range = this.edit.word_range_at(this.edit.cursor());
                    this.edit.set_selection(range, false);
                    cx.notify();
                }
                if event.click_count() >= 3 {
                    this.edit.select_all();
                    cx.notify();
                }
            }))
            .child(TextFieldElement {
                field: cx.entity(),
                focused,
            })
    }
}

impl EntityInputHandler for TextField {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let text = self.edit.content();
        let byte_range = byte_range_from_utf16(text, &range);
        *adjusted = Some(utf16_range_from_bytes(text, &byte_range));
        Some(text[byte_range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let text = self.edit.content();
        Some(UTF16Selection {
            range: utf16_range_from_bytes(text, &self.edit.selection()),
            reversed: self.edit.selection_is_reversed(),
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked
            .as_ref()
            .map(|range| utf16_range_from_bytes(self.edit.content(), range))
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.marked = None;
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .map(|range| byte_range_from_utf16(self.edit.content(), &range))
            .or_else(|| self.marked.clone())
            .unwrap_or_else(|| self.edit.selection());
        self.edit.replace(range, text);
        self.emit_changed(window, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        new_text: &str,
        new_selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .map(|range| byte_range_from_utf16(self.edit.content(), &range))
            .or_else(|| self.marked.clone())
            .unwrap_or_else(|| self.edit.selection());
        let start = range.start;
        self.edit.replace(range, new_text);
        self.marked = (!new_text.is_empty()).then_some(start..start + new_text.len());
        if let Some(selected) = new_selected {
            let selected = byte_range_from_utf16(new_text, &selected);
            self.edit
                .set_selection(start + selected.start..start + selected.end, false);
        }
        self.caret_visible = true;
        cx.emit(TextFieldEvent::Changed(self.edit.content().to_string()));
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        // Good enough to place a candidate window against the field.
        Some(element_bounds)
    }

    fn character_index_for_point(
        &mut self,
        _: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        Some(utf16_offset_for_byte(
            self.edit.content(),
            self.edit.cursor(),
        ))
    }
}

/// Paints the field and owns the shaping its hit-testing depends on.
struct TextFieldElement {
    field: Entity<TextField>,
    focused: bool,
}

/// One hard line of the buffer, already soft-wrapped to the field's width.
struct FieldLine {
    line: WrappedLine,
    /// Byte offset of this line's first character in the displayed text.
    start: usize,
    /// Visual rows this line occupies once wrapped.
    rows: usize,
}

pub struct TextFieldLayout {
    lines: Vec<FieldLine>,
    /// `false` while the placeholder is showing, so the caret is not drawn
    /// against text the buffer does not contain.
    content_line: bool,
    scroll: Point<Pixels>,
    selection: Range<usize>,
    caret: usize,
}

impl TextFieldLayout {
    /// Where a displayed offset sits, relative to the text origin.
    fn position_for_offset(&self, offset: usize, line_height: Pixels) -> Point<Pixels> {
        let mut y = px(0.);
        for line in &self.lines {
            let end = line.start + line.line.len();
            if offset <= end {
                let local = offset.saturating_sub(line.start);
                if let Some(position) = line.line.position_for_index(local, line_height) {
                    return point(position.x, y + position.y);
                }
            }
            y += line_height * line.rows as f32;
        }
        point(px(0.), (y - line_height).max(px(0.)))
    }

    /// Byte ranges of each visual row of a wrapped line, in line-local offsets.
    fn row_ranges(line: &WrappedLine) -> Vec<Range<usize>> {
        let mut starts = vec![0usize];
        for boundary in line.wrap_boundaries.iter() {
            let run = &line.unwrapped_layout.runs[boundary.run_ix];
            starts.push(run.glyphs[boundary.glyph_ix].index);
        }
        let len = line.len();
        (0..starts.len())
            .map(|index| starts[index]..starts.get(index + 1).copied().unwrap_or(len))
            .collect()
    }
}

impl IntoElement for TextFieldElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextFieldElement {
    type RequestLayoutState = ();
    type PrepaintState = TextFieldLayout;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let multi_line = self.field.read(cx).multi_line;
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        // A wrapped field fills the box its owner sized; a single-line one is
        // exactly one line tall wherever it is placed.
        style.size.height = if multi_line {
            relative(1.).into()
        } else {
            window.line_height().into()
        };
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> TextFieldLayout {
        let field = self.field.read(cx);
        let multi_line = field.multi_line;
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line_height = window.line_height();

        let content = display_text(&field.edit, field.masked);
        let showing_placeholder = content.is_empty() && !field.placeholder.is_empty();
        let text: SharedString = if showing_placeholder {
            field.placeholder.clone()
        } else {
            content.into()
        };
        let color = if showing_placeholder {
            style.color.opacity(0.55)
        } else {
            style.color
        };
        let run = TextRun {
            len: text.len(),
            font: style.font(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let wrap_width = multi_line.then_some(bounds.size.width);
        let shaped = window
            .text_system()
            .shape_text(text.clone(), font_size, &[run], wrap_width, None)
            .unwrap_or_default();

        // `shape_text` splits on newlines, dropping the separator, so line
        // starts have to account for the byte it consumed.
        let mut lines = Vec::with_capacity(shaped.len());
        let mut start = 0usize;
        for line in shaped {
            let rows = line.wrap_boundaries.len() + 1;
            let len = line.len();
            lines.push(FieldLine { line, start, rows });
            start += len + 1;
        }
        if lines.is_empty() {
            return TextFieldLayout {
                lines,
                content_line: false,
                scroll: point(px(0.), px(0.)),
                selection: 0..0,
                caret: 0,
            };
        }

        let caret = display_offset(&field.edit, field.masked, field.edit.cursor());
        let selection = field.edit.selection();
        let selection = display_offset(&field.edit, field.masked, selection.start)
            ..display_offset(&field.edit, field.masked, selection.end);

        let mut layout = TextFieldLayout {
            lines,
            content_line: !showing_placeholder,
            scroll: point(field.scroll_x, field.scroll_y),
            selection,
            caret,
        };

        // Keep the caret inside the viewport: sideways when the field does not
        // wrap, downward when it does.
        let caret_position = layout.position_for_offset(caret, line_height);
        let mut scroll_x = field.scroll_x;
        let mut scroll_y = field.scroll_y;
        if multi_line {
            scroll_x = px(0.);
            let rows: usize = layout.lines.iter().map(|line| line.rows).sum();
            let content_height = line_height * rows as f32;
            if caret_position.y + line_height - scroll_y > bounds.size.height {
                scroll_y = caret_position.y + line_height - bounds.size.height;
            }
            if caret_position.y < scroll_y {
                scroll_y = caret_position.y;
            }
            let overflow = (content_height - bounds.size.height).max(px(0.));
            scroll_y = scroll_y.clamp(px(0.), overflow);
        } else {
            let width = layout
                .lines
                .first()
                .map(|line| line.line.unwrapped_layout.width)
                .unwrap_or_default();
            if caret_position.x - scroll_x > bounds.size.width {
                scroll_x = caret_position.x - bounds.size.width;
            }
            if caret_position.x < scroll_x {
                scroll_x = caret_position.x;
            }
            let overflow = (width - bounds.size.width).max(px(0.));
            scroll_x = scroll_x.clamp(px(0.), overflow);
        }
        if scroll_x != field.scroll_x || scroll_y != field.scroll_y {
            self.field.update(cx, |field, _| {
                field.scroll_x = scroll_x;
                field.scroll_y = scroll_y;
            });
        }
        layout.scroll = point(scroll_x, scroll_y);
        layout
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        layout: &mut TextFieldLayout,
        window: &mut Window,
        cx: &mut App,
    ) {
        let (blinking_on, focus) = {
            let field = self.field.read(cx);
            (field.caret_visible, field.focus.clone())
        };
        let selection_color = window.text_style().color.opacity(0.25);
        let caret_color = window.text_style().color;
        let caret_visible = self.focused && blinking_on && layout.content_line;
        let line_height = window.line_height();
        let origin = point(
            bounds.origin.x - layout.scroll.x,
            bounds.origin.y - layout.scroll.y,
        );

        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
            let mut y = px(0.);
            for line in &layout.lines {
                let line_origin = point(origin.x, origin.y + y);
                if !layout.selection.is_empty() {
                    // Paint the selection row by row: a wrapped line's highlight
                    // is one quad per visual row, not one across the whole line.
                    for (row, range) in TextFieldLayout::row_ranges(&line.line)
                        .into_iter()
                        .enumerate()
                    {
                        let row_start = line.start + range.start;
                        let row_end = line.start + range.end;
                        let start = layout.selection.start.max(row_start);
                        let end = layout.selection.end.min(row_end);
                        if start >= end {
                            continue;
                        }
                        let row_left = line.line.unwrapped_layout.x_for_index(range.start);
                        let x0 =
                            line.line.unwrapped_layout.x_for_index(start - line.start) - row_left;
                        let x1 =
                            line.line.unwrapped_layout.x_for_index(end - line.start) - row_left;
                        window.paint_quad(fill(
                            Bounds::new(
                                point(line_origin.x + x0, line_origin.y + line_height * row as f32),
                                size(x1 - x0, line_height),
                            ),
                            selection_color,
                        ));
                    }
                }

                line.line
                    .paint(
                        line_origin,
                        line_height,
                        gpui::TextAlign::Left,
                        None,
                        window,
                        cx,
                    )
                    .ok();
                y += line_height * line.rows as f32;
            }

            if caret_visible {
                let position = layout.position_for_offset(layout.caret, line_height);
                window.paint_quad(fill(
                    Bounds::new(
                        point(origin.x + position.x, origin.y + position.y),
                        size(px(1.5), line_height),
                    ),
                    caret_color,
                ));
            }
        });

        // Route platform composition and clipboard at the field's own bounds.
        if self.focused {
            window.handle_input(
                &focus,
                ElementInputHandler::new(bounds, self.field.clone()),
                cx,
            );
        }

        // Pointer positioning reuses the shaping above rather than re-measuring.
        let entity = self.field.clone();
        let scroll = layout.scroll;
        let text_origin = bounds.origin;
        let hit = Rc::new(std::mem::take(&mut layout.lines));
        let hit_lines = hit.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase != gpui::DispatchPhase::Bubble
                || event.button != MouseButton::Left
                || !bounds.contains(&event.position)
            {
                return;
            }
            entity.update(cx, |field, cx| {
                let offset =
                    offset_at(&hit_lines, event.position, text_origin, scroll, line_height);
                let offset = buffer_offset(&field.edit, field.masked, offset);
                if event.modifiers.shift {
                    let anchor = field.edit.selection();
                    let anchor = if offset < anchor.start {
                        anchor.end
                    } else {
                        anchor.start
                    };
                    field
                        .edit
                        .set_selection(anchor.min(offset)..anchor.max(offset), offset < anchor);
                } else {
                    field.edit.set_cursor(offset);
                }
                field.restart_blink(window, cx);
                cx.notify();
            });
            window.refresh();
        });

        let entity = self.field.clone();
        let hit_lines = hit.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
            if phase != gpui::DispatchPhase::Bubble {
                return;
            }
            if !entity.read(cx).selecting {
                return;
            }
            entity.update(cx, |field, cx| {
                let offset =
                    offset_at(&hit_lines, event.position, text_origin, scroll, line_height);
                let offset = buffer_offset(&field.edit, field.masked, offset);
                let anchor = field.edit.selection();
                let anchor = if field.edit.selection_is_reversed() {
                    anchor.end
                } else {
                    anchor.start
                };
                field
                    .edit
                    .set_selection(anchor.min(offset)..anchor.max(offset), offset < anchor);
                cx.notify();
            });
        });

        let entity = self.field.clone();
        window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
            if phase == gpui::DispatchPhase::Bubble {
                entity.update(cx, |field, _| field.selecting = false);
            }
        });
    }
}

/// Displayed offset under a window-space point.
fn offset_at(
    lines: &[FieldLine],
    position: Point<Pixels>,
    origin: Point<Pixels>,
    scroll: Point<Pixels>,
    line_height: Pixels,
) -> usize {
    let local = point(
        position.x - origin.x + scroll.x,
        position.y - origin.y + scroll.y,
    );
    let mut y = px(0.);
    for (index, line) in lines.iter().enumerate() {
        let height = line_height * line.rows as f32;
        if local.y < y + height || index + 1 == lines.len() {
            let in_line = point(local.x, (local.y - y).max(px(0.)));
            let offset = line
                .line
                .closest_index_for_position(in_line, line_height)
                .unwrap_or_else(|index| index);
            return line.start + offset;
        }
        y += height;
    }
    lines
        .last()
        .map(|line| line.start + line.line.len())
        .unwrap_or(0)
}

/// The chrome around a [`TextField`]: border, background, focus ring.
///
/// Kept separate from the widget so callers can place the field inside their own
/// row without inheriting a box they did not want.
pub fn text_field_box(
    id: impl Into<ElementId>,
    field: &Entity<TextField>,
    palette: ThemePalette,
    focused: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .h(px(30.))
        .px_2()
        .flex()
        .items_center()
        .rounded_md()
        .border_1()
        .border_color(rgb(if focused {
            palette.primary
        } else {
            palette.border
        }))
        .bg(rgb(palette.input))
        .text_color(rgb(palette.text))
        .child(div().flex_1().min_w_0().child(field.clone()))
}

fn byte_range_from_utf16(text: &str, range: &Range<usize>) -> Range<usize> {
    let start = byte_for_utf16_offset(text, range.start);
    let end = byte_for_utf16_offset(text, range.end.max(range.start));
    start..end
}

fn byte_for_utf16_offset(text: &str, offset: usize) -> usize {
    let mut utf16 = 0;
    for (index, c) in text.char_indices() {
        if utf16 >= offset {
            return index;
        }
        utf16 += c.len_utf16();
    }
    text.len()
}

fn utf16_range_from_bytes(text: &str, range: &Range<usize>) -> Range<usize> {
    utf16_offset_for_byte(text, range.start)..utf16_offset_for_byte(text, range.end)
}

fn utf16_offset_for_byte(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())]
        .chars()
        .map(char::len_utf16)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::{
        buffer_offset, display_offset, display_text, prepare_pasted_text, utf16_offset_for_byte,
    };
    use nyaterm_core::TextEdit;

    #[test]
    fn masking_maps_offsets_between_the_buffer_and_the_bullets() {
        let edit = TextEdit::new("aé中");
        assert_eq!(display_text(&edit, true), "•••");

        // Buffer offsets are 0,1,3,6; bullet offsets step by three.
        assert_eq!(display_offset(&edit, true, 0), 0);
        assert_eq!(display_offset(&edit, true, 1), 3);
        assert_eq!(display_offset(&edit, true, 3), 6);
        assert_eq!(display_offset(&edit, true, 6), 9);

        assert_eq!(buffer_offset(&edit, true, 0), 0);
        assert_eq!(buffer_offset(&edit, true, 3), 1);
        assert_eq!(buffer_offset(&edit, true, 6), 3);
        assert_eq!(buffer_offset(&edit, true, 9), 6);
    }

    #[test]
    fn unmasked_offsets_are_snapped_onto_char_boundaries() {
        let edit = TextEdit::new("中文");
        assert_eq!(buffer_offset(&edit, false, 1), 0);
        assert_eq!(buffer_offset(&edit, false, 3), 3);
    }

    #[test]
    fn utf16_offsets_account_for_surrogate_pairs() {
        // An emoji is one char but two UTF-16 units, which is what the IME counts.
        assert_eq!(utf16_offset_for_byte("a🙂b", 5), 3);
        assert_eq!(utf16_offset_for_byte("a🙂b", 1), 1);
    }

    #[test]
    fn single_line_paste_flattens_normalized_line_endings() {
        assert_eq!(
            prepare_pasted_text("first\r\nsecond\rthird\nfourth", false),
            "first second third fourth"
        );
    }

    #[test]
    fn multi_line_paste_preserves_normalized_line_endings() {
        assert_eq!(
            prepare_pasted_text("first\r\nsecond\rthird\nfourth", true),
            "first\nsecond\nthird\nfourth"
        );
    }
}
