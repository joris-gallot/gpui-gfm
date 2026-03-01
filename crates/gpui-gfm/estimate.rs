//! Height estimation for markdown blocks.
//!
//! Used for virtual scrolling — lets the UI pre-allocate space for markdown
//! content without rendering it. All values are in logical pixels.

use crate::types::*;

// Layout constants (match the render module).
const BASE_BLOCK_GAP_PX: f32 = 8.0;
const HEADING_EXTRA_TOP_MARGIN_PX: f32 = 10.0;
const LIST_ITEM_GAP_PX: f32 = 4.0;
const LIST_LEFT_PADDING_PX: f32 = 10.0;
const LIST_MARKER_GAP_PX: f32 = 4.0;
const INDENT_PER_LEVEL_PX: f32 = 12.0;
const CHAR_WIDTH_PX: f32 = 8.8;
const MIN_WRAP_COLUMNS: usize = 8;
const CODE_BLOCK_VERTICAL_CHROME_PX: f32 = 18.0; // padding top + bottom + border
const CODE_BLOCK_MAX_HEIGHT_PX: f32 = 400.0;

/// Estimate the rendered height of a markdown source string.
pub fn estimate_markdown_height_px(source: &str, wrap_columns: usize, line_height_px: f32) -> f32 {
  let parsed = crate::parse::parse_markdown(source);
  estimate_parsed_markdown_height_px(&parsed, wrap_columns, line_height_px)
}

/// Estimate the rendered height of a pre-parsed markdown document.
pub fn estimate_parsed_markdown_height_px(
  parsed: &ParsedMarkdown,
  wrap_columns: usize,
  line_height_px: f32,
) -> f32 {
  estimate_blocks_height_px(parsed.blocks(), wrap_columns, line_height_px, 0)
}

/// Estimate height of a list of blocks.
fn estimate_blocks_height_px(
  blocks: &[Block],
  wrap_columns: usize,
  line_height_px: f32,
  indent: usize,
) -> f32 {
  if blocks.is_empty() {
    return 0.0;
  }

  let mut total = 0.0f32;
  for (ix, block) in blocks.iter().enumerate() {
    if ix > 0 {
      total += BASE_BLOCK_GAP_PX;
    }
    if ix > 0 && matches!(block, Block::Heading { .. }) {
      total += HEADING_EXTRA_TOP_MARGIN_PX;
    }
    total += estimate_block_height_px(block, wrap_columns, line_height_px, indent);
  }

  total.max(line_height_px)
}

/// Estimate height of a single block.
fn estimate_block_height_px(
  block: &Block,
  wrap_columns: usize,
  line_height_px: f32,
  indent: usize,
) -> f32 {
  match block {
    Block::Paragraph(inlines) => {
      let cols = wrap_columns_for_indent(wrap_columns, indent);
      estimate_inline_lines(inlines, cols) as f32 * line_height_px
    }
    Block::Heading { content, .. } => {
      let cols = wrap_columns_for_indent(wrap_columns, indent);
      let lines = estimate_inline_lines(content, cols);
      lines as f32 * (line_height_px * 1.4) // headings are larger
    }
    Block::List(list) => estimate_list_height_px(list, wrap_columns, line_height_px, indent),
    Block::CodeBlock(code) => {
      let lines = code.value.lines().count().max(1);
      let content_height = lines as f32 * line_height_px + CODE_BLOCK_VERTICAL_CHROME_PX;
      content_height.min(CODE_BLOCK_MAX_HEIGHT_PX)
    }
    Block::BlockQuote(children) => {
      estimate_blocks_height_px(children, wrap_columns, line_height_px, indent + 1)
    }
    Block::ThematicBreak => 1.0 + BASE_BLOCK_GAP_PX,
    Block::Table(table) => estimate_table_height_px(table, line_height_px),
    Block::Details(details) => {
      estimate_details_height_px(details, wrap_columns, line_height_px, indent)
    }
    Block::Aligned { blocks, .. } => {
      estimate_blocks_height_px(blocks, wrap_columns, line_height_px, indent)
    }
  }
}

/// Estimate list height.
fn estimate_list_height_px(
  list: &List,
  wrap_columns: usize,
  line_height_px: f32,
  indent: usize,
) -> f32 {
  if list.items.is_empty() {
    return 0.0;
  }

  let indent_cols =
    ((LIST_LEFT_PADDING_PX + LIST_MARKER_GAP_PX + 14.0) / CHAR_WIDTH_PX).ceil() as usize;
  let item_wrap_columns = wrap_columns_for_indent(wrap_columns, indent)
    .saturating_sub(indent_cols)
    .max(MIN_WRAP_COLUMNS);

  let mut total = 0.0f32;
  for (ix, item) in list.items.iter().enumerate() {
    if ix > 0 {
      total += LIST_ITEM_GAP_PX;
    }
    total += estimate_blocks_height_px(&item.blocks, item_wrap_columns, line_height_px, 0);
  }

  total.max(line_height_px)
}

/// Estimate details height.
fn estimate_details_height_px(
  details: &Details,
  wrap_columns: usize,
  line_height_px: f32,
  indent: usize,
) -> f32 {
  let summary_cols = wrap_columns_for_indent(wrap_columns, indent)
    .saturating_sub(3) // chevron icon width
    .max(MIN_WRAP_COLUMNS);
  let summary_height =
    estimate_inline_lines(&details.summary, summary_cols) as f32 * line_height_px;

  if !details.open {
    return summary_height;
  }

  let body = estimate_blocks_height_px(&details.blocks, wrap_columns, line_height_px, indent + 1);
  summary_height + BASE_BLOCK_GAP_PX + body
}

/// Estimate table height.
fn estimate_table_height_px(table: &Table, line_height_px: f32) -> f32 {
  let row_height = line_height_px + 16.0; // padding
  let header_height = row_height;
  let body_height = table.rows.len() as f32 * (row_height + 1.0); // +1 for border
  header_height + 2.0 + body_height // +2 for header border
}

/// Compute effective wrap columns at a given indent level.
fn wrap_columns_for_indent(base_wrap_columns: usize, indent: usize) -> usize {
  let indent_columns = ((indent as f32 * INDENT_PER_LEVEL_PX) / CHAR_WIDTH_PX).ceil() as usize;
  base_wrap_columns
    .saturating_sub(indent_columns)
    .max(MIN_WRAP_COLUMNS)
}

/// Estimate how many visual lines a set of inlines will occupy.
fn estimate_inline_lines(inlines: &[Inline], wrap_columns: usize) -> usize {
  let text = inline_to_plain_text(inlines);
  if text.is_empty() {
    return 1;
  }

  let mut lines = 0usize;
  for line in text.split('\n') {
    lines += estimate_wrapped_text_lines(line, wrap_columns);
  }
  lines.max(1)
}

/// Estimate how many visual lines a single text line occupies after wrapping.
fn estimate_wrapped_text_lines(line: &str, wrap_columns: usize) -> usize {
  let wrap_columns = wrap_columns.max(1);
  if line.is_empty() {
    return 1;
  }

  let char_count = line.chars().count();
  ((char_count + wrap_columns - 1) / wrap_columns).max(1)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn empty_blocks_height_zero() {
    assert_eq!(estimate_blocks_height_px(&[], 80, 20.0, 0), 0.0);
  }

  #[test]
  fn single_paragraph_height() {
    let blocks = vec![Block::Paragraph(vec![Inline::Text("Hello".into())])];
    let h = estimate_blocks_height_px(&blocks, 80, 20.0, 0);
    assert!(h >= 20.0);
  }

  #[test]
  fn code_block_height_capped() {
    let code = "x\n".repeat(1000);
    let blocks = vec![Block::CodeBlock(CodeBlock {
      lang: None,
      value: code,
    })];
    let h = estimate_blocks_height_px(&blocks, 80, 20.0, 0);
    assert!(h <= CODE_BLOCK_MAX_HEIGHT_PX);
  }

  #[test]
  fn thematic_break_height() {
    let blocks = vec![Block::ThematicBreak];
    let h = estimate_blocks_height_px(&blocks, 80, 20.0, 0);
    assert!(h > 0.0);
    assert!(h <= 20.0);
  }

  #[test]
  fn details_closed_shorter_than_open() {
    let details_closed = Details {
      summary: vec![Inline::Text("Summary".into())],
      blocks: vec![Block::Paragraph(vec![Inline::Text("Body content".into())])],
      open: false,
    };
    let details_open = Details {
      summary: vec![Inline::Text("Summary".into())],
      blocks: vec![Block::Paragraph(vec![Inline::Text("Body content".into())])],
      open: true,
    };

    let h_closed = estimate_details_height_px(&details_closed, 80, 20.0, 0);
    let h_open = estimate_details_height_px(&details_open, 80, 20.0, 0);
    assert!(h_open > h_closed);
  }

  #[test]
  fn wrapping_increases_line_count() {
    let short = estimate_wrapped_text_lines("hello", 80);
    let long = estimate_wrapped_text_lines(&"x".repeat(200), 80);
    assert_eq!(short, 1);
    assert!(long > 1);
  }
}
