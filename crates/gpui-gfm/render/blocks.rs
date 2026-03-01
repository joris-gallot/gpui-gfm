//! Block-level rendering.

use gpui::{AnyElement, App, div, prelude::*, px};

use crate::types::*;

use super::MarkdownRenderOptions;
use super::code_block::render_code_block;
use super::inline::render_inline_text;
use super::table::render_table;

/// Render a list of blocks as a vertical stack.
pub fn render_blocks(
  blocks: &[Block],
  options: &MarkdownRenderOptions,
  indent: usize,
  cx: &App,
) -> AnyElement {
  let mut container = div().flex().flex_col().gap_2();

  for (ix, block) in blocks.iter().enumerate() {
    let block_element = render_block(block, options, indent, cx);
    let block_element = if ix > 0 && matches!(block, Block::Heading { .. }) {
      div().mt(px(10.0)).child(block_element).into_any_element()
    } else {
      block_element
    };
    container = container.child(block_element);
  }

  if indent > 0 {
    container = container.pl(px(12.0 * indent as f32));
  }

  container.into_any_element()
}

/// Render a single block.
fn render_block(
  block: &Block,
  options: &MarkdownRenderOptions,
  indent: usize,
  cx: &App,
) -> AnyElement {
  let theme = options.theme();

  match block {
    Block::Paragraph(inlines) => div()
      .whitespace_normal()
      .text_sm()
      .text_color(theme.foreground)
      .child(render_inline_text(inlines, options, cx))
      .into_any_element(),

    Block::Heading { level, content } => {
      let el = div().text_color(theme.foreground);
      let el = match level {
        1 => el.text_3xl().font_weight(gpui::FontWeight::BOLD),
        2 => el.text_2xl().font_weight(gpui::FontWeight::SEMIBOLD),
        3 => el.text_xl().font_weight(gpui::FontWeight::SEMIBOLD),
        4 => el.text_lg().font_weight(gpui::FontWeight::MEDIUM),
        5 => el.text_base().font_weight(gpui::FontWeight::MEDIUM),
        _ => el.text_sm().font_weight(gpui::FontWeight::MEDIUM),
      };
      el.child(render_inline_text(content, options, cx))
        .into_any_element()
    }

    Block::List(list) => render_list(list, options, indent, cx),

    Block::CodeBlock(code) => render_code_block(code, options, cx),

    Block::BlockQuote(children) => div()
      .border_l_2()
      .border_color(theme.muted_foreground)
      .pl(px(8.0))
      .child(render_blocks(children, options, indent + 1, cx))
      .into_any_element(),

    Block::ThematicBreak => div()
      .h(px(1.0))
      .bg(theme.border)
      .rounded_md()
      .into_any_element(),

    Block::Table(table) => render_table(table, options, cx),

    Block::Details(details) => render_details(details, options, indent, cx),

    Block::Aligned { center, blocks } => {
      if *center {
        let mut aligned = div().flex().flex_col().w_full().min_w_0().gap_2();
        for block in blocks {
          aligned = aligned.child(
            div().flex().w_full().min_w_0().justify_center().child(
              div()
                .text_center()
                .min_w_0()
                .child(render_block(block, options, indent, cx)),
            ),
          );
        }
        aligned.into_any_element()
      } else {
        render_blocks(blocks, options, indent, cx)
      }
    }
  }
}

/// Render a list.
fn render_list(
  list: &List,
  options: &MarkdownRenderOptions,
  _indent: usize,
  cx: &App,
) -> AnyElement {
  let theme = options.theme();
  let mut container = div()
    .flex()
    .flex_col()
    .w_full()
    .min_w_0()
    .gap_1()
    .pl(px(10.0));
  let start = list.start.unwrap_or(1);

  for (ix, item) in list.items.iter().enumerate() {
    let bullet = if list.ordered {
      format!("{}.", start + ix as u64)
    } else if item.checked == Some(true) {
      "☑".to_string()
    } else if item.checked == Some(false) {
      "☐".to_string()
    } else {
      "•".to_string()
    };

    let item_content = render_list_item_blocks(&item.blocks, options, cx);

    let row = div()
      .flex()
      .items_start()
      .w_full()
      .min_w_0()
      .child(
        div()
          .flex_none()
          .text_sm()
          .text_color(theme.foreground)
          .pr(px(4.0))
          .child(bullet),
      )
      .child(div().min_w_0().flex_1().child(item_content));

    container = container.child(row);
  }

  container.into_any_element()
}

/// Render the blocks within a list item.
fn render_list_item_blocks(
  blocks: &[Block],
  options: &MarkdownRenderOptions,
  cx: &App,
) -> AnyElement {
  let mut container = div().flex().flex_col().w_full().min_w_0().gap_2();
  for block in blocks {
    container = container.child(render_block(block, options, 0, cx));
  }
  container.into_any_element()
}

/// Render a `<details>` block.
///
/// For now, always render as open (interactive toggle requires stateful Element).
/// TODO: Add stateful toggle.
fn render_details(
  details: &Details,
  options: &MarkdownRenderOptions,
  indent: usize,
  cx: &App,
) -> AnyElement {
  let theme = options.theme();

  let summary_el = div()
    .flex()
    .items_center()
    .gap_1()
    .text_sm()
    .font_weight(gpui::FontWeight::MEDIUM)
    .text_color(theme.foreground)
    .child("▶ ")
    .child(render_inline_text(&details.summary, options, cx));

  let mut container = div().flex().flex_col().gap_2().child(summary_el);

  // Render body if open
  if details.open {
    container = container.child(div().pl(px(16.0)).child(render_blocks(
      &details.blocks,
      options,
      indent + 1,
      cx,
    )));
  }

  container.into_any_element()
}
