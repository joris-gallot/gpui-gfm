use std::ops::Range;

use gpui::{
  App, Bounds, ClipboardItem, Context, CursorStyle, ElementId, ElementInputHandler, Entity,
  EntityInputHandler, FocusHandle, Focusable, GlobalElementId, LayoutId, MouseButton,
  MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point, ShapedLine, SharedString,
  Style, TextRun, UTF16Selection, Window, actions, div, fill, point, prelude::*, px, relative,
  rgba, size,
};
use unicode_segmentation::*;

actions!(
  text_input,
  [
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    SelectLeft,
    SelectRight,
    SelectAll,
    Home,
    End,
    ShowCharacterPalette,
    Paste,
    Cut,
    Copy,
    Enter,
    WordLeft,
    WordRight,
    SelectWordLeft,
    SelectWordRight,
    DeleteWordLeft,
    LineStart,
    LineEnd,
    SelectToLineStart,
    SelectToLineEnd,
    DeleteToLineStart,
  ]
);

type OnEnterCallback = Box<dyn Fn(&mut Window, &mut App)>;

pub struct TextInput {
  pub focus_handle: FocusHandle,
  pub content: SharedString,
  placeholder: Option<SharedString>,
  selected_range: Range<usize>,
  selection_reversed: bool,
  marked_range: Option<Range<usize>>,
  last_layout: Option<Vec<ShapedLine>>,
  last_bounds: Option<Bounds<Pixels>>,
  scroll_offset: f32,
  is_selecting: bool,
  on_enter: Option<OnEnterCallback>,
}

impl TextInput {
  pub fn new(cx: &mut Context<Self>, initial_content: String) -> Self {
    Self {
      focus_handle: cx.focus_handle(),
      content: initial_content.into(),
      placeholder: None,
      selected_range: 0..0,
      selection_reversed: false,
      marked_range: None,
      last_layout: None,
      last_bounds: None,
      scroll_offset: 0.0,
      is_selecting: false,
      on_enter: None,
    }
  }

  /// Set a callback invoked when Enter is pressed (instead of inserting a newline).
  pub fn on_enter(mut self, callback: impl Fn(&mut Window, &mut App) + 'static) -> Self {
    self.on_enter = Some(Box::new(callback));
    self
  }

  /// Set placeholder text shown when content is empty.
  pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
    self.placeholder = Some(text.into());
    self
  }

  /// Replace the text content.
  pub fn set_content(&mut self, text: String, cx: &mut Context<Self>) {
    self.content = text.into();
    self.selected_range = 0..0;
    self.marked_range = None;
    cx.notify();
  }

  /// Return the current text content.
  pub fn text(&self) -> &str {
    &self.content
  }

  fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
    if self.selected_range.is_empty() {
      self.move_to(self.previous_boundary(self.cursor_offset()), cx);
    } else {
      self.move_to(self.selected_range.start, cx);
    }
  }

  fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
    if self.selected_range.is_empty() {
      self.move_to(self.next_boundary(self.selected_range.end), cx);
    } else {
      self.move_to(self.selected_range.end, cx);
    }
  }

  fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
    let (row, col) = self.offset_to_row_col(self.cursor_offset());
    if row > 0 {
      let new_offset = self.row_col_to_offset(row - 1, col);
      self.move_to(new_offset, cx);
    } else {
      self.move_to(0, cx);
    }
  }

  fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
    let (row, col) = self.offset_to_row_col(self.cursor_offset());
    let line_count = self.content.split('\n').count();
    if row + 1 < line_count {
      let new_offset = self.row_col_to_offset(row + 1, col);
      self.move_to(new_offset, cx);
    } else {
      self.move_to(self.content.len(), cx);
    }
  }

  fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
    self.select_to(self.previous_boundary(self.cursor_offset()), cx);
  }

  fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
    self.select_to(self.next_boundary(self.cursor_offset()), cx);
  }

  fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
    self.move_to(0, cx);
    self.select_to(self.content.len(), cx);
  }

  fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
    // Move to start of current line
    let (row, _) = self.offset_to_row_col(self.cursor_offset());
    let offset = self.row_col_to_offset(row, 0);
    self.move_to(offset, cx);
  }

  fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
    // Move to end of current line
    let (row, _) = self.offset_to_row_col(self.cursor_offset());
    let line_len = self.content.split('\n').nth(row).map_or(0, |l| l.len());
    let offset = self.row_col_to_offset(row, line_len);
    self.move_to(offset, cx);
  }

  fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
    if self.selected_range.is_empty() {
      self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }
    self.replace_text_in_range(None, "", window, cx);
  }

  fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
    if self.selected_range.is_empty() {
      self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }
    self.replace_text_in_range(None, "", window, cx);
  }

  fn enter(&mut self, _: &Enter, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(on_enter) = self.on_enter.as_ref() {
      on_enter(window, &mut **cx);
    } else {
      self.replace_text_in_range(None, "\n", window, cx);
    }
  }

  // ── Word navigation (Alt+Arrow) ──

  fn word_left(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
    self.move_to(self.previous_word_boundary(self.cursor_offset()), cx);
  }

  fn word_right(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
    self.move_to(self.next_word_boundary(self.cursor_offset()), cx);
  }

  fn select_word_left(&mut self, _: &SelectWordLeft, _: &mut Window, cx: &mut Context<Self>) {
    self.select_to(self.previous_word_boundary(self.cursor_offset()), cx);
  }

  fn select_word_right(&mut self, _: &SelectWordRight, _: &mut Window, cx: &mut Context<Self>) {
    self.select_to(self.next_word_boundary(self.cursor_offset()), cx);
  }

  fn delete_word_left(&mut self, _: &DeleteWordLeft, window: &mut Window, cx: &mut Context<Self>) {
    if self.selected_range.is_empty() {
      self.select_to(self.previous_word_boundary(self.cursor_offset()), cx);
    }
    self.replace_text_in_range(None, "", window, cx);
  }

  // ── Line navigation (Cmd+Arrow) ──

  fn line_start(&mut self, _: &LineStart, _: &mut Window, cx: &mut Context<Self>) {
    let offset = self.line_start_offset(self.cursor_offset());
    self.move_to(offset, cx);
  }

  fn line_end(&mut self, _: &LineEnd, _: &mut Window, cx: &mut Context<Self>) {
    let offset = self.line_end_offset(self.cursor_offset());
    self.move_to(offset, cx);
  }

  fn select_to_line_start(
    &mut self,
    _: &SelectToLineStart,
    _: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let offset = self.line_start_offset(self.cursor_offset());
    self.select_to(offset, cx);
  }

  fn select_to_line_end(&mut self, _: &SelectToLineEnd, _: &mut Window, cx: &mut Context<Self>) {
    let offset = self.line_end_offset(self.cursor_offset());
    self.select_to(offset, cx);
  }

  fn delete_to_line_start(
    &mut self,
    _: &DeleteToLineStart,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    if self.selected_range.is_empty() {
      let start = self.line_start_offset(self.cursor_offset());
      self.select_to(start, cx);
    }
    self.replace_text_in_range(None, "", window, cx);
  }

  fn on_mouse_down(
    &mut self,
    event: &MouseDownEvent,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    self.is_selecting = true;
    if event.modifiers.shift {
      self.select_to(self.index_for_mouse_position(event.position), cx);
    } else {
      self.move_to(self.index_for_mouse_position(event.position), cx);
    }
  }

  fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
    self.is_selecting = false;
  }

  fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
    if self.is_selecting {
      self.select_to(self.index_for_mouse_position(event.position), cx);
    }
  }

  fn show_character_palette(
    &mut self,
    _: &ShowCharacterPalette,
    window: &mut Window,
    _: &mut Context<Self>,
  ) {
    window.show_character_palette();
  }

  fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
      self.replace_text_in_range(None, &text, window, cx);
    }
  }

  fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
    if !self.selected_range.is_empty() {
      cx.write_to_clipboard(ClipboardItem::new_string(
        self.content[self.selected_range.clone()].to_string(),
      ));
    }
  }

  fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
    if !self.selected_range.is_empty() {
      cx.write_to_clipboard(ClipboardItem::new_string(
        self.content[self.selected_range.clone()].to_string(),
      ));
      self.replace_text_in_range(None, "", window, cx);
    }
  }

  fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
    self.selected_range = offset..offset;
    cx.notify();
  }

  fn cursor_offset(&self) -> usize {
    if self.selection_reversed {
      self.selected_range.start
    } else {
      self.selected_range.end
    }
  }

  fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
    if self.selection_reversed {
      self.selected_range.start = offset;
    } else {
      self.selected_range.end = offset;
    }
    if self.selected_range.end < self.selected_range.start {
      self.selection_reversed = !self.selection_reversed;
      self.selected_range = self.selected_range.end..self.selected_range.start;
    }
    cx.notify();
  }

  fn previous_boundary(&self, offset: usize) -> usize {
    if offset == 0 {
      return 0;
    }
    // Handle newline as boundary
    let bytes = self.content.as_bytes();
    if offset > 0 && bytes.get(offset - 1) == Some(&b'\n') {
      return offset - 1;
    }
    self.content[..offset]
      .grapheme_indices(true)
      .next_back()
      .map(|(idx, _)| idx)
      .unwrap_or(0)
  }

  fn next_boundary(&self, offset: usize) -> usize {
    if offset >= self.content.len() {
      return self.content.len();
    }
    let bytes = self.content.as_bytes();
    if bytes.get(offset) == Some(&b'\n') {
      return offset + 1;
    }
    self.content[offset..]
      .grapheme_indices(true)
      .nth(1)
      .map(|(idx, _)| offset + idx)
      .unwrap_or(self.content.len())
  }

  /// Find the start of the previous word from `offset`.
  fn previous_word_boundary(&self, offset: usize) -> usize {
    if offset == 0 {
      return 0;
    }
    let before = &self.content[..offset];
    // Skip trailing whitespace/punctuation, then skip the word itself
    let mut pos = before.len();
    // Skip whitespace backwards
    for ch in before.chars().rev() {
      if ch.is_whitespace() || ch.is_ascii_punctuation() {
        pos -= ch.len_utf8();
      } else {
        break;
      }
    }
    // Skip word chars backwards
    for ch in self.content[..pos].chars().rev() {
      if ch.is_whitespace() || ch.is_ascii_punctuation() {
        break;
      }
      pos -= ch.len_utf8();
    }
    pos
  }

  /// Find the end of the next word from `offset`.
  fn next_word_boundary(&self, offset: usize) -> usize {
    if offset >= self.content.len() {
      return self.content.len();
    }
    let after = &self.content[offset..];
    let mut pos = offset;
    let mut chars = after.chars();
    // Skip current word chars
    for ch in chars.by_ref() {
      if ch.is_whitespace() || ch.is_ascii_punctuation() {
        pos += ch.len_utf8();
        break;
      }
      pos += ch.len_utf8();
    }
    // Skip whitespace/punctuation
    for ch in chars {
      if ch.is_whitespace() || ch.is_ascii_punctuation() {
        pos += ch.len_utf8();
      } else {
        break;
      }
    }
    pos
  }

  /// Byte offset of the start of the current line.
  fn line_start_offset(&self, offset: usize) -> usize {
    self.content[..offset].rfind('\n').map_or(0, |p| p + 1)
  }

  /// Byte offset of the end of the current line (before the newline).
  fn line_end_offset(&self, offset: usize) -> usize {
    self.content[offset..]
      .find('\n')
      .map_or(self.content.len(), |p| offset + p)
  }

  /// Convert a byte offset to (row, col) in the text.
  fn offset_to_row_col(&self, offset: usize) -> (usize, usize) {
    let mut row = 0;
    let mut col = 0;
    for (i, ch) in self.content.char_indices() {
      if i >= offset {
        break;
      }
      if ch == '\n' {
        row += 1;
        col = 0;
      } else {
        col += ch.len_utf8();
      }
    }
    if offset > 0 && offset <= self.content.len() {
      // If offset is exactly at a position, recalculate col
      let line_start = self.content[..offset].rfind('\n').map_or(0, |p| p + 1);
      col = offset - line_start;
    }
    (row, col)
  }

  /// Convert (row, col) to byte offset.
  fn row_col_to_offset(&self, row: usize, col: usize) -> usize {
    let mut offset = 0;
    for (i, line) in self.content.split('\n').enumerate() {
      if i == row {
        return offset + col.min(line.len());
      }
      offset += line.len() + 1; // +1 for '\n'
    }
    self.content.len()
  }

  fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
    if self.content.is_empty() {
      return 0;
    }
    let (Some(bounds), Some(lines)) = (self.last_bounds.as_ref(), self.last_layout.as_ref()) else {
      return 0;
    };

    let line_height = 20.0; // Approximate, matches our text_sm
    let y_offset: f32 = (position.y - bounds.top()).into();
    let y_in_content = y_offset + self.scroll_offset;
    let row = (y_in_content / line_height).max(0.0) as usize;
    let row = row.min(lines.len().saturating_sub(1));

    // Find byte offset of this row's start
    let mut row_start_offset = 0usize;
    for (i, line_text) in self.content.split('\n').enumerate() {
      if i == row {
        break;
      }
      row_start_offset += line_text.len() + 1;
    }

    if let Some(line) = lines.get(row) {
      let col_offset = line.closest_index_for_x(position.x - bounds.left());
      row_start_offset + col_offset
    } else {
      self.content.len()
    }
  }

  fn offset_from_utf16(&self, offset: usize) -> usize {
    let mut utf8_offset = 0;
    let mut utf16_count = 0;
    for ch in self.content.chars() {
      if utf16_count >= offset {
        break;
      }
      utf16_count += ch.len_utf16();
      utf8_offset += ch.len_utf8();
    }
    utf8_offset
  }

  fn offset_to_utf16(&self, offset: usize) -> usize {
    let mut utf16_offset = 0;
    let mut utf8_count = 0;
    for ch in self.content.chars() {
      if utf8_count >= offset {
        break;
      }
      utf8_count += ch.len_utf8();
      utf16_offset += ch.len_utf16();
    }
    utf16_offset
  }

  fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
    self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
  }

  fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
    self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
  }
}

impl EntityInputHandler for TextInput {
  fn text_for_range(
    &mut self,
    range_utf16: Range<usize>,
    actual_range: &mut Option<Range<usize>>,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<String> {
    let range = self.range_from_utf16(&range_utf16);
    actual_range.replace(self.range_to_utf16(&range));
    Some(self.content[range].to_string())
  }

  fn selected_text_range(
    &mut self,
    _ignore_disabled_input: bool,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<UTF16Selection> {
    Some(UTF16Selection {
      range: self.range_to_utf16(&self.selected_range),
      reversed: self.selection_reversed,
    })
  }

  fn marked_text_range(
    &self,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<Range<usize>> {
    self
      .marked_range
      .as_ref()
      .map(|range| self.range_to_utf16(range))
  }

  fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
    self.marked_range = None;
  }

  fn replace_text_in_range(
    &mut self,
    range_utf16: Option<Range<usize>>,
    new_text: &str,
    _: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let range = range_utf16
      .as_ref()
      .map(|r| self.range_from_utf16(r))
      .or(self.marked_range.clone())
      .unwrap_or(self.selected_range.clone());

    self.content =
      (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..]).into();
    self.selected_range = range.start + new_text.len()..range.start + new_text.len();
    self.marked_range.take();
    cx.notify();
  }

  fn replace_and_mark_text_in_range(
    &mut self,
    range_utf16: Option<Range<usize>>,
    new_text: &str,
    new_selected_range_utf16: Option<Range<usize>>,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let range = range_utf16
      .as_ref()
      .map(|r| self.range_from_utf16(r))
      .or(self.marked_range.clone())
      .unwrap_or(self.selected_range.clone());

    self.content =
      (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..]).into();
    if !new_text.is_empty() {
      self.marked_range = Some(range.start..range.start + new_text.len());
    } else {
      self.marked_range = None;
    }
    self.selected_range = new_selected_range_utf16
      .as_ref()
      .map(|r| self.range_from_utf16(r))
      .map(|new_range| new_range.start + range.start..new_range.end + range.end)
      .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
    cx.notify();
  }

  fn bounds_for_range(
    &mut self,
    range_utf16: Range<usize>,
    bounds: Bounds<Pixels>,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<Bounds<Pixels>> {
    let range = self.range_from_utf16(&range_utf16);
    let (row, _) = self.offset_to_row_col(range.start);
    let lines = self.last_layout.as_ref()?;
    let line = lines.get(row)?;
    let line_start = self.row_col_to_offset(row, 0);
    let local_start = range.start - line_start;
    let local_end = (range.end - line_start).min(line.text.len());
    let line_height = px(20.0);
    let y = bounds.top() + line_height * row as f32 - px(self.scroll_offset);
    Some(Bounds::from_corners(
      point(bounds.left() + line.x_for_index(local_start), y),
      point(bounds.left() + line.x_for_index(local_end), y + line_height),
    ))
  }

  fn character_index_for_point(
    &mut self,
    point: gpui::Point<Pixels>,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<usize> {
    let idx = self.index_for_mouse_position(point);
    Some(self.offset_to_utf16(idx))
  }
}

impl Focusable for TextInput {
  fn focus_handle(&self, _: &App) -> FocusHandle {
    self.focus_handle.clone()
  }
}

impl Render for TextInput {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .flex()
      .flex_col()
      .w_full()
      .key_context("TextInput")
      .track_focus(&self.focus_handle(cx))
      .cursor(CursorStyle::IBeam)
      .on_action(cx.listener(Self::backspace))
      .on_action(cx.listener(Self::delete))
      .on_action(cx.listener(Self::left))
      .on_action(cx.listener(Self::right))
      .on_action(cx.listener(Self::up))
      .on_action(cx.listener(Self::down))
      .on_action(cx.listener(Self::select_left))
      .on_action(cx.listener(Self::select_right))
      .on_action(cx.listener(Self::select_all))
      .on_action(cx.listener(Self::home))
      .on_action(cx.listener(Self::end))
      .on_action(cx.listener(Self::enter))
      .on_action(cx.listener(Self::word_left))
      .on_action(cx.listener(Self::word_right))
      .on_action(cx.listener(Self::select_word_left))
      .on_action(cx.listener(Self::select_word_right))
      .on_action(cx.listener(Self::delete_word_left))
      .on_action(cx.listener(Self::line_start))
      .on_action(cx.listener(Self::line_end))
      .on_action(cx.listener(Self::select_to_line_start))
      .on_action(cx.listener(Self::select_to_line_end))
      .on_action(cx.listener(Self::delete_to_line_start))
      .on_action(cx.listener(Self::show_character_palette))
      .on_action(cx.listener(Self::paste))
      .on_action(cx.listener(Self::cut))
      .on_action(cx.listener(Self::copy))
      .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
      .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
      .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
      .on_mouse_move(cx.listener(Self::on_mouse_move))
      .child(TextElement { input: cx.entity() })
  }
}

// ── Custom element for multiline text rendering ──

pub struct TextElement {
  input: Entity<TextInput>,
}

pub struct PrepaintState {
  lines: Vec<ShapedLine>,
  cursor: Option<PaintQuad>,
  selections: Vec<PaintQuad>,
  placeholder: Option<ShapedLine>,
}

impl IntoElement for TextElement {
  type Element = Self;
  fn into_element(self) -> Self::Element {
    self
  }
}

impl Element for TextElement {
  type RequestLayoutState = ();
  type PrepaintState = PrepaintState;

  fn id(&self) -> Option<ElementId> {
    None
  }

  fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
    None
  }

  fn request_layout(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector_id: Option<&gpui::InspectorElementId>,
    window: &mut Window,
    cx: &mut App,
  ) -> (LayoutId, Self::RequestLayoutState) {
    let input = self.input.read(cx);
    let line_count = input.content.split('\n').count().max(1);
    let line_height = window.line_height();

    let mut style = Style::default();
    style.size.width = relative(1.).into();
    style.size.height = (line_height * line_count as f32).into();
    (window.request_layout(style, [], cx), ())
  }

  fn prepaint(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector_id: Option<&gpui::InspectorElementId>,
    bounds: Bounds<Pixels>,
    _request_layout: &mut Self::RequestLayoutState,
    window: &mut Window,
    cx: &mut App,
  ) -> Self::PrepaintState {
    let input = self.input.read(cx);
    let content = input.content.clone();
    let selected_range = input.selected_range.clone();
    let cursor_offset = input.cursor_offset();
    let placeholder_text = input.placeholder.clone();
    let style = window.text_style();
    let font_size = style.font_size.to_pixels(window.rem_size());
    let line_height = window.line_height();

    let text_color = style.color;

    // Shape each line
    let text_lines: Vec<String> = content.split('\n').map(|s| s.to_string()).collect();
    let mut shaped_lines = Vec::with_capacity(text_lines.len());

    for line_text in &text_lines {
      let display: SharedString = if line_text.is_empty() {
        " ".into() // Ensure empty lines have height
      } else {
        line_text.clone().into()
      };
      let run = TextRun {
        len: display.len(),
        font: style.font(),
        color: text_color,
        background_color: None,
        underline: None,
        strikethrough: None,
      };
      let shaped = window
        .text_system()
        .shape_line(display, font_size, &[run], None);
      shaped_lines.push(shaped);
    }

    // Compute cursor position
    let (cursor_row, cursor_col) = {
      let mut row = 0;
      let mut remaining = cursor_offset;
      for line_text in &text_lines {
        if remaining <= line_text.len() {
          break;
        }
        remaining -= line_text.len() + 1; // +1 for '\n'
        row += 1;
      }
      let col = if row < text_lines.len() {
        remaining.min(text_lines[row].len())
      } else {
        0
      };
      (row.min(text_lines.len() - 1), col)
    };

    let cursor_quad = if selected_range.is_empty() {
      if let Some(line) = shaped_lines.get(cursor_row) {
        let x = line.x_for_index(cursor_col);
        Some(fill(
          Bounds::new(
            point(
              bounds.left() + x,
              bounds.top() + line_height * cursor_row as f32,
            ),
            size(px(2.), line_height),
          ),
          gpui::hsla(210.0 / 360.0, 1.0, 0.5, 0.7),
        ))
      } else {
        None
      }
    } else {
      None
    };

    // Compute selection quads
    let mut selections = Vec::new();
    if !selected_range.is_empty() {
      let mut offset = 0usize;
      for (row, line_text) in text_lines.iter().enumerate() {
        let line_start = offset;
        let line_end = offset + line_text.len();

        let sel_start = selected_range.start.max(line_start);
        let sel_end = selected_range.end.min(line_end);

        if sel_start < sel_end {
          if let Some(shaped) = shaped_lines.get(row) {
            let x_start = shaped.x_for_index(sel_start - line_start);
            let x_end = shaped.x_for_index(sel_end - line_start);
            selections.push(fill(
              Bounds::from_corners(
                point(
                  bounds.left() + x_start,
                  bounds.top() + line_height * row as f32,
                ),
                point(
                  bounds.left() + x_end,
                  bounds.top() + line_height * (row + 1) as f32,
                ),
              ),
              rgba(0x3388ff40),
            ));
          }
        }
        // If selection spans into next line, highlight to end
        if selected_range.end > line_end && selected_range.start <= line_end {
          if let Some(shaped) = shaped_lines.get(row) {
            let x_start = if sel_start <= line_end {
              shaped.x_for_index((sel_start.max(line_start)) - line_start)
            } else {
              px(0.)
            };
            // Only add if we didn't already add one for this row
            if sel_start >= sel_end {
              let x_end = shaped.x_for_index(line_text.len());
              selections.push(fill(
                Bounds::from_corners(
                  point(
                    bounds.left() + x_start,
                    bounds.top() + line_height * row as f32,
                  ),
                  point(
                    bounds.left() + x_end + px(4.),
                    bounds.top() + line_height * (row + 1) as f32,
                  ),
                ),
                rgba(0x3388ff40),
              ));
            }
          }
        }

        offset = line_end + 1; // +1 for '\n'
      }
    }

    // Shape placeholder if content is empty
    let placeholder_line = if content.is_empty() {
      placeholder_text.map(|text| {
        let run = TextRun {
          len: text.len(),
          font: style.font(),
          color: gpui::hsla(0., 0., 0.5, 0.5),
          background_color: None,
          underline: None,
          strikethrough: None,
        };
        window
          .text_system()
          .shape_line(text, font_size, &[run], None)
      })
    } else {
      None
    };

    PrepaintState {
      lines: shaped_lines,
      cursor: cursor_quad,
      selections,
      placeholder: placeholder_line,
    }
  }

  fn paint(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector_id: Option<&gpui::InspectorElementId>,
    bounds: Bounds<Pixels>,
    _request_layout: &mut Self::RequestLayoutState,
    prepaint: &mut Self::PrepaintState,
    window: &mut Window,
    cx: &mut App,
  ) {
    let focus_handle = self.input.read(cx).focus_handle.clone();
    window.handle_input(
      &focus_handle,
      ElementInputHandler::new(bounds, self.input.clone()),
      cx,
    );

    // Paint selections
    for sel in prepaint.selections.drain(..) {
      window.paint_quad(sel);
    }

    // Paint lines
    let line_height = window.line_height();

    // Paint placeholder if present
    if let Some(placeholder) = &prepaint.placeholder {
      let origin = point(bounds.left(), bounds.top());
      placeholder
        .paint(origin, line_height, gpui::TextAlign::Left, None, window, cx)
        .ok();
    }

    for (row, line) in prepaint.lines.iter().enumerate() {
      let origin = point(bounds.left(), bounds.top() + line_height * row as f32);
      line
        .paint(origin, line_height, gpui::TextAlign::Left, None, window, cx)
        .ok();
    }

    // Paint cursor
    if focus_handle.is_focused(window) {
      if let Some(cursor) = prepaint.cursor.take() {
        window.paint_quad(cursor);
      }
    }

    self.input.update(cx, |input, _cx| {
      input.last_layout = Some(std::mem::take(&mut prepaint.lines));
      input.last_bounds = Some(bounds);
    });
  }
}
