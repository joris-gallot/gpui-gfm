//! Code block rendering.

use gpui::{AnyElement, App, ClipboardItem, Font, MouseButton, SharedString, div, prelude::*, px};

use crate::types::CodeBlock;

use super::MarkdownRenderOptions;

/// Maximum height (px) before vertical scroll kicks in.
const CODE_BLOCK_MAX_HEIGHT_PX: f32 = 500.0;
const CODE_BLOCK_PADDING_X_PX: f32 = 12.0;
const CODE_BLOCK_PADDING_TOP_PX: f32 = 8.0;
const CODE_BLOCK_PADDING_BOTTOM_PX: f32 = 8.0;

/// Render a code block.
pub fn render_code_block(
  code: &CodeBlock,
  options: &MarkdownRenderOptions,
  _cx: &App,
) -> AnyElement {
  let theme = options.theme();

  // Prepare display text: strip trailing newline
  let display_value = code_block_display_value(code);
  let text: SharedString = display_value.clone().into();

  // Language label
  let lang_label = code.lang.as_deref().unwrap_or("");

  // Outer container with group for hover-reveal of copy button
  let container_id: SharedString =
    format!("md-code-container-{:x}", code as *const CodeBlock as usize).into();

  let mut container = div()
    .id(container_id)
    .group("code-block")
    .w_full()
    .min_w_0()
    .rounded_md()
    .border_1()
    .border_color(theme.border)
    .bg(theme.code_background)
    .overflow_hidden();

  // Copy button — positioned inside the code area wrapper (below header)
  let copy_btn_id: SharedString = format!("md-copy-{:x}", code as *const CodeBlock as usize).into();
  let clipboard_value = display_value.clone();
  let hover_bg = theme.border;

  let copy_button = div()
    .id(copy_btn_id)
    .absolute()
    .top_1()
    .right_1()
    .px_2()
    .py(px(2.0))
    .rounded_md()
    .text_xs()
    .text_color(theme.muted_foreground)
    .bg(theme.code_background)
    .border_1()
    .border_color(theme.border)
    .cursor_pointer()
    .opacity(0.0)
    .group_hover("code-block", |s| s.opacity(1.0))
    .hover(move |s| s.bg(hover_bg))
    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
      cx.write_to_clipboard(ClipboardItem::new_string(clipboard_value.clone()));
    })
    .child("Copy");

  // Language header if present
  if !lang_label.is_empty() {
    container = container.child(
      div()
        .px(px(CODE_BLOCK_PADDING_X_PX))
        .py_1()
        .text_xs()
        .text_color(theme.muted_foreground)
        .border_b_1()
        .border_color(theme.border)
        .child(lang_label.to_string()),
    );
  }

  // Code content — needs an id to support scrolling
  let code_id: SharedString = format!("md-code-{:x}", code as *const CodeBlock as usize).into();
  let code_font = Font {
    family: theme.code_font_family.clone(),
    ..Default::default()
  };

  let mut code_area = div()
    .id(code_id)
    .px(px(CODE_BLOCK_PADDING_X_PX))
    .pt(px(CODE_BLOCK_PADDING_TOP_PX))
    .pb(px(CODE_BLOCK_PADDING_BOTTOM_PX))
    .text_sm()
    .text_color(theme.foreground)
    .font(code_font)
    .whitespace_nowrap()
    .overflow_x_scroll();

  // Cap height and enable Y scroll (no-op for short blocks).
  // Stop scroll-wheel propagation so the parent container doesn't scroll
  // simultaneously (scroll chaining).
  if !options.expand_code_blocks {
    code_area = code_area
      .max_h(px(CODE_BLOCK_MAX_HEIGHT_PX))
      .overflow_y_scroll()
      .on_scroll_wheel(|_, _, cx| {
        cx.stop_propagation();
      });
  }

  code_area = code_area.child(text);

  // Wrap code area + copy button in a relative container so the button
  // is positioned relative to the code area (below the header).
  let code_wrapper = div().relative().child(code_area).child(copy_button);

  container.child(code_wrapper).into_any_element()
}

/// Prepare the display text for a code block.
fn code_block_display_value(code: &CodeBlock) -> String {
  let mut value = code.value.clone();
  // Strip single trailing newline (comrak always adds one)
  if value.ends_with('\n') {
    value.pop();
  }
  // Expand tabs to 4 spaces
  value = expand_tabs(&value);
  value
}

/// Expand tab characters to spaces (4-space tab stops).
fn expand_tabs(text: &str) -> String {
  let mut result = String::with_capacity(text.len());
  let mut col = 0usize;
  for ch in text.chars() {
    match ch {
      '\t' => {
        let spaces = 4 - (col % 4);
        for _ in 0..spaces {
          result.push(' ');
        }
        col += spaces;
      }
      '\n' => {
        result.push('\n');
        col = 0;
      }
      _ => {
        result.push(ch);
        col += 1;
      }
    }
  }
  result
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn strips_trailing_newline() {
    let code = CodeBlock {
      lang: Some("rust".into()),
      value: "fn main() {}\n".into(),
    };
    assert_eq!(code_block_display_value(&code), "fn main() {}");
  }

  #[test]
  fn expands_tabs() {
    assert_eq!(expand_tabs("\tfoo"), "    foo");
    assert_eq!(expand_tabs("a\tb"), "a   b");
    assert_eq!(expand_tabs("ab\tc"), "ab  c");
    assert_eq!(expand_tabs("abc\td"), "abc d");
    assert_eq!(expand_tabs("abcd\te"), "abcd    e");
  }

  #[test]
  fn preserves_content_without_trailing_newline() {
    let code = CodeBlock {
      lang: None,
      value: "no newline".into(),
    };
    assert_eq!(code_block_display_value(&code), "no newline");
  }

  #[test]
  fn clipboard_content_matches_display() {
    // The clipboard should get the same content as what's displayed
    let code = CodeBlock {
      lang: Some("rust".into()),
      value: "fn main() {\n\tprintln!(\"hello\");\n}\n".into(),
    };
    let display = code_block_display_value(&code);
    // Trailing newline stripped, tabs expanded
    assert_eq!(display, "fn main() {\n    println!(\"hello\");\n}");
  }
}
