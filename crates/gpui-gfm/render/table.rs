//! Table rendering.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};

use crate::types::*;

use super::MarkdownRenderOptions;
use super::inline::render_inline_text;

/// Minimum column width.
const TABLE_CELL_MIN_WIDTH_PX: f32 = 64.0;
/// Horizontal padding per cell.
const TABLE_CELL_HORIZONTAL_PADDING_PX: f32 = 24.0;
/// Approximate character width for column sizing.
const TABLE_INLINE_CHAR_WIDTH_PX: f32 = 7.2;

/// Render a GFM table.
pub fn render_table(table: &Table, options: &MarkdownRenderOptions, cx: &App) -> AnyElement {
  let theme = options.theme();
  let column_count = table
    .rows
    .iter()
    .fold(table.headers.len(), |count, row| count.max(row.len()))
    .max(1);
  let column_widths = compute_column_widths(table, column_count);

  // Header row
  let mut header_row = div().flex().bg(theme.accent.opacity(0.15));
  for (col, width) in column_widths.iter().enumerate().take(column_count) {
    let cell = table
      .headers
      .get(col)
      .map_or(&[][..], |cell| cell.as_slice());
    header_row = header_row.child(
      div()
        .w(px(*width))
        .px_3()
        .py_2()
        .when(col + 1 < column_count, |this| {
          this.border_r_1().border_color(theme.border)
        })
        .child(
          div()
            .text_sm()
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(theme.foreground)
            .whitespace_nowrap()
            .child(render_inline_text(cell, options, cx)),
        ),
    );
  }

  // Body rows
  let mut body = div().flex().flex_col();
  for row in &table.rows {
    let mut row_el = div().flex().border_t_1().border_color(theme.border);
    for (col, width) in column_widths.iter().enumerate().take(column_count) {
      let cell = row.get(col).map_or(&[][..], |cell| cell.as_slice());
      row_el = row_el.child(
        div()
          .w(px(*width))
          .px_3()
          .py_2()
          .when(col + 1 < column_count, |this| {
            this.border_r_1().border_color(theme.border)
          })
          .child(
            div()
              .text_sm()
              .text_color(theme.foreground)
              .whitespace_nowrap()
              .child(render_inline_text(cell, options, cx)),
          ),
      );
    }
    body = body.child(row_el);
  }

  // Scroll container
  let table_id: SharedString = format!("md-table-{:x}", table as *const Table as usize).into();

  div()
    .id(table_id)
    .w_full()
    .min_w_0()
    .overflow_x_scroll()
    .child(
      div()
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .overflow_hidden()
        .child(div().flex().flex_col().child(header_row).child(body)),
    )
    .into_any_element()
}

/// Compute column widths based on content.
fn compute_column_widths(table: &Table, column_count: usize) -> Vec<f32> {
  let mut widths = vec![TABLE_CELL_MIN_WIDTH_PX; column_count];

  for (col, width) in widths.iter_mut().enumerate().take(column_count) {
    // Check header
    if let Some(cell) = table.headers.get(col) {
      *width = (*width).max(estimate_cell_width(cell));
    }
    // Check all rows
    for row in &table.rows {
      if let Some(cell) = row.get(col) {
        *width = (*width).max(estimate_cell_width(cell));
      }
    }
  }

  widths
}

/// Estimate the pixel width of a table cell's content.
fn estimate_cell_width(inlines: &[Inline]) -> f32 {
  let text = inline_to_plain_text(inlines);
  let char_width = text.len() as f32 * TABLE_INLINE_CHAR_WIDTH_PX;
  (char_width + TABLE_CELL_HORIZONTAL_PADDING_PX).max(TABLE_CELL_MIN_WIDTH_PX)
}
