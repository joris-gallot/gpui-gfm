//! Code reference preview card rendering.
//!
//! Renders a [`GithubCodeReferencePreview`] as a bordered card with a header
//! (file label + line range) and a scrollable code snippet body.

use gpui::{
  AnyElement, App, Font, FontStyle, FontWeight, MouseButton, SharedString, div, prelude::*, px,
};

use crate::github::{GithubCodeReferencePreview, short_github_reference};

use super::MarkdownRenderOptions;

/// Render a code reference preview as a card element.
pub fn render_code_reference_card(
  preview: &GithubCodeReferencePreview,
  options: &MarkdownRenderOptions,
  _cx: &App,
) -> AnyElement {
  let theme = options.theme();
  let url = preview.url.clone();
  let on_link = options.on_link.clone();

  // Build labels.
  let file_label = format!("{}/{}", preview.repo, preview.path);
  let line_label = if preview.start_line == preview.end_line {
    format!(
      "Line {} in {}",
      preview.start_line,
      short_github_reference(&preview.reference)
    )
  } else {
    format!(
      "Lines {}-{} in {}",
      preview.start_line,
      preview.end_line,
      short_github_reference(&preview.reference)
    )
  };

  // Build snippet rows.
  let code_font = Font {
    family: theme.code_font_family.clone(),
    weight: FontWeight::NORMAL,
    style: FontStyle::Normal,
    ..Default::default()
  };

  let mut snippet_rows = div().flex().flex_col().gap(px(2.0));

  if preview.snippets.is_empty() {
    snippet_rows = snippet_rows.child(
      div()
        .flex()
        .items_center()
        .gap_2()
        .child(
          div()
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .text_color(theme.muted_foreground)
            .child(preview.start_line.to_string()),
        )
        .child(
          div()
            .font(code_font.clone())
            .text_sm()
            .whitespace_nowrap()
            .text_color(theme.foreground)
            .child(""),
        ),
    );
  } else {
    for (offset, snippet) in preview.snippets.iter().enumerate() {
      let line_number = preview.start_line + offset;
      let snippet_text: SharedString = snippet.to_string().into();
      snippet_rows = snippet_rows.child(
        div()
          .flex()
          .items_center()
          .gap_2()
          .child(
            div()
              .flex()
              .justify_end()
              .text_xs()
              .font_weight(FontWeight::MEDIUM)
              .text_color(theme.muted_foreground)
              .min_w(px(28.0))
              .flex_shrink_0()
              .child(line_number.to_string()),
          )
          .child(
            div()
              .font(code_font.clone())
              .text_sm()
              .whitespace_nowrap()
              .text_color(theme.foreground)
              .child(snippet_text),
          ),
      );
    }
  }

  // Build the card.
  div()
    .my_1()
    .border_1()
    .border_color(theme.border)
    .rounded_md()
    .overflow_hidden()
    // Header: file label + line range, clickable.
    .child(
      div()
        .id("code-preview-header")
        .group("code-preview-header")
        .bg(theme.code_background)
        .border_b_1()
        .border_color(theme.border)
        .px(px(12.0))
        .py(px(6.0))
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
          cx.stop_propagation();
          if let Some(handler) = &on_link {
            handler(url.as_ref(), window, cx);
          } else {
            cx.open_url(url.as_ref());
          }
        })
        .child(
          div()
            .flex()
            .flex_col()
            .child(
              div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.link)
                .group_hover("code-preview-header", |s| s.underline())
                .child(file_label),
            )
            .child(
              div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(line_label),
            ),
        ),
    )
    // Body: scrollable code snippet.
    .child(
      div()
        .id("code-preview-body")
        .w_full()
        .min_w_0()
        .max_h(px(400.0))
        .overflow_scroll()
        .px(px(12.0))
        .py(px(6.0))
        .bg(theme.background)
        .child(snippet_rows),
    )
    .into_any_element()
}
