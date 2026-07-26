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
use std::time::Duration;

use gpui::{
    App, Bounds, ClickEvent, Context, CursorStyle, Element, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, EventEmitter, FocusHandle, Focusable, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    Point, Render, ShapedLine, SharedString, Style, TextRun, UTF16Selection, Window, div, fill,
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
    /// Horizontal scroll, so a caret past the right edge stays visible.
    scroll_x: Pixels,
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
            scroll_x: px(0.),
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
                    // A single-line field: paste the first line only, rather
                    // than silently storing a newline nothing can display.
                    let text = text.replace(['\n', '\r'], " ");
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
            // Navigation and dialog keys belong to the owner. Some of them do
            // carry a `key_char` — Tab is "	" — so they have to be named, or
            // they would be typed into the buffer instead.
            "tab" | "enter" | "escape" | "up" | "down" | "pageup" | "pagedown" => return false,
            _ => {
                // Everything else is text, unless a modifier claims it or the
                // key produces none.
                if accel || keystroke.modifiers.function {
                    return false;
                }
                let Some(input) = keystroke.key_char.as_deref().filter(|s| !s.is_empty()) else {
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

pub struct TextFieldLayout {
    line: ShapedLine,
    /// `None` while the placeholder is showing, so the caret is not drawn
    /// against text the buffer does not contain.
    content_line: bool,
    scroll_x: Pixels,
    selection: Range<usize>,
    caret: usize,
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
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
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
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());

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
        let line = window
            .text_system()
            .shape_line(text, font_size, &[run], None);

        let caret = display_offset(&field.edit, field.masked, field.edit.cursor());
        let selection = field.edit.selection();
        let selection = display_offset(&field.edit, field.masked, selection.start)
            ..display_offset(&field.edit, field.masked, selection.end);

        // Keep the caret inside the viewport; the field never wraps.
        let caret_x = line.x_for_index(caret);
        let mut scroll_x = field.scroll_x;
        if caret_x - scroll_x > bounds.size.width {
            scroll_x = caret_x - bounds.size.width;
        }
        if caret_x < scroll_x {
            scroll_x = caret_x;
        }
        let overflow = (line.width - bounds.size.width).max(px(0.));
        scroll_x = scroll_x.clamp(px(0.), overflow);
        if scroll_x != field.scroll_x {
            self.field.update(cx, |field, _| field.scroll_x = scroll_x);
        }

        TextFieldLayout {
            line,
            content_line: !showing_placeholder,
            scroll_x,
            selection,
            caret,
        }
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
        let caret_visible = self.focused && blinking_on && layout.content_line;
        let line_height = window.line_height();
        let origin = point(bounds.origin.x - layout.scroll_x, bounds.origin.y);

        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
            if !layout.selection.is_empty() {
                let start = layout.line.x_for_index(layout.selection.start);
                let end = layout.line.x_for_index(layout.selection.end);
                window.paint_quad(fill(
                    Bounds::new(
                        point(origin.x + start, bounds.origin.y),
                        size(end - start, line_height),
                    ),
                    selection_color,
                ));
            }

            layout.line.paint(origin, line_height, window, cx).ok();

            if caret_visible {
                let x = origin.x + layout.line.x_for_index(layout.caret);
                window.paint_quad(fill(
                    Bounds::new(point(x, bounds.origin.y), size(px(1.5), line_height)),
                    window.text_style().color,
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
        let line = layout.line.clone();
        let scroll_x = layout.scroll_x;
        let left = bounds.origin.x;
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase != gpui::DispatchPhase::Bubble
                || event.button != MouseButton::Left
                || !bounds.contains(&event.position)
            {
                return;
            }
            entity.update(cx, |field, cx| {
                let display = line.closest_index_for_x(event.position.x - left + scroll_x);
                let offset = buffer_offset(&field.edit, field.masked, display);
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
        let line = layout.line.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
            if phase != gpui::DispatchPhase::Bubble {
                return;
            }
            if !entity.read(cx).selecting {
                return;
            }
            entity.update(cx, |field, cx| {
                let display = line.closest_index_for_x(event.position.x - left + scroll_x);
                let offset = buffer_offset(&field.edit, field.masked, display);
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
    use super::{buffer_offset, display_offset, display_text, utf16_offset_for_byte};
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
}
