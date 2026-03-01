//! Inline text rendering — converts `Vec<Inline>` into styled GPUI text.

use gpui::{
  AnyElement, App, Font, FontStyle, FontWeight, SharedString, StrikethroughStyle, StyledText,
  TextRun, UnderlineStyle, div, prelude::*, px,
};

use crate::types::*;

use super::MarkdownRenderOptions;

/// Render a list of inlines as a GPUI text element.
pub fn render_inline_text(
  inlines: &[Inline],
  options: &MarkdownRenderOptions,
  cx: &App,
) -> AnyElement {
  // Build plain text and style spans
  let mut text = String::new();
  let mut runs: Vec<TextRun> = Vec::new();

  flatten_inlines(inlines, &mut text, &mut runs, &InlineContext {
    bold: false,
    italic: false,
    strikethrough: false,
    code: false,
    link: false,
  }, options, cx);

  if text.is_empty() {
    return div().into_any_element();
  }

  let shared_text: SharedString = text.into();
  StyledText::new(shared_text)
    .with_runs(runs)
    .into_any_element()
}

/// Context tracking the current inline formatting state.
struct InlineContext {
  bold: bool,
  italic: bool,
  strikethrough: bool,
  code: bool,
  link: bool,
}

/// Recursively flatten inlines into a plain text string + TextRun spans.
fn flatten_inlines(
  inlines: &[Inline],
  text: &mut String,
  runs: &mut Vec<TextRun>,
  ctx: &InlineContext,
  options: &MarkdownRenderOptions,
  cx: &App,
) {
  let theme = options.theme();

  for inline in inlines {
    match inline {
      Inline::Text(value) => {
        let start = text.len();
        text.push_str(value);
        let end = text.len();
        if end > start {
          runs.push(make_text_run(start..end, ctx, theme));
        }
      }

      Inline::Code(value) => {
        let start = text.len();
        text.push_str(value);
        let end = text.len();
        if end > start {
          let code_ctx = InlineContext {
            code: true,
            ..*ctx
          };
          runs.push(make_text_run(start..end, &code_ctx, theme));
        }
      }

      Inline::SoftBreak => {
        let start = text.len();
        text.push(' ');
        runs.push(make_text_run(start..text.len(), ctx, theme));
      }

      Inline::HardBreak => {
        let start = text.len();
        text.push('\n');
        runs.push(make_text_run(start..text.len(), ctx, theme));
      }

      Inline::Strong(children) => {
        let bold_ctx = InlineContext {
          bold: true,
          ..*ctx
        };
        flatten_inlines(children, text, runs, &bold_ctx, options, cx);
      }

      Inline::Emphasis(children) => {
        let italic_ctx = InlineContext {
          italic: true,
          ..*ctx
        };
        flatten_inlines(children, text, runs, &italic_ctx, options, cx);
      }

      Inline::Strikethrough(children) => {
        let strike_ctx = InlineContext {
          strikethrough: true,
          ..*ctx
        };
        flatten_inlines(children, text, runs, &strike_ctx, options, cx);
      }

      Inline::Link { content, .. } => {
        let link_ctx = InlineContext {
          link: true,
          ..*ctx
        };
        flatten_inlines(content, text, runs, &link_ctx, options, cx);
      }

      Inline::Image { alt, .. } => {
        // For now, render alt text inline. Image rendering will be added later.
        if !alt.is_empty() {
          let start = text.len();
          text.push_str(alt);
          let end = text.len();
          runs.push(make_text_run(start..end, ctx, theme));
        }
      }
    }
  }
}

/// Create a `TextRun` with the appropriate styling for the current context.
fn make_text_run(
  range: std::ops::Range<usize>,
  ctx: &InlineContext,
  theme: &super::MarkdownTheme,
) -> TextRun {
  let len = range.end - range.start;

  let font_weight = if ctx.bold {
    FontWeight::BOLD
  } else {
    FontWeight::NORMAL
  };

  let font_style = if ctx.italic {
    FontStyle::Italic
  } else {
    FontStyle::Normal
  };

  let color = if ctx.link {
    theme.link
  } else if ctx.code {
    theme.foreground
  } else {
    theme.foreground
  };

  let underline = if ctx.link {
    UnderlineStyle {
      thickness: px(1.0),
      color: Some(theme.link),
      wavy: false,
    }
  } else {
    UnderlineStyle::default()
  };

  let strikethrough = if ctx.strikethrough {
    StrikethroughStyle {
      thickness: px(1.0),
      color: Some(theme.foreground),
    }
  } else {
    StrikethroughStyle::default()
  };

  let run = TextRun {
    len,
    font: if ctx.code {
      Font {
        family: theme.code_font_family.clone(),
        weight: font_weight,
        style: font_style,
        ..Default::default()
      }
    } else {
      Font {
        weight: font_weight,
        style: font_style,
        ..Default::default()
      }
    },
    color,
    underline: Some(underline),
    strikethrough: Some(strikethrough),
    background_color: if ctx.code {
      Some(theme.code_background)
    } else {
      None
    },
  };

  run
}
