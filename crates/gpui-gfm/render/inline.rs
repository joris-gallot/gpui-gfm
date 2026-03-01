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

  flatten_inlines(
    inlines,
    &mut text,
    &mut runs,
    &InlineContext {
      bold: false,
      italic: false,
      strikethrough: false,
      code: false,
      link: false,
    },
    options,
    cx,
  );

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
          let code_ctx = InlineContext { code: true, ..*ctx };
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
        let bold_ctx = InlineContext { bold: true, ..*ctx };
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
        let link_ctx = InlineContext { link: true, ..*ctx };
        flatten_inlines(content, text, runs, &link_ctx, options, cx);
      }

      Inline::Image { alt, url, .. } => {
        // Resolve the URL (relative → absolute if base_url is set).
        let _resolved_url = resolve_url(url, options);
        // For now, render alt text inline. Full image rendering in étape 8.
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

/// Resolve a potentially relative URL against the `image_base_url`.
///
/// - Absolute URLs (`http://`, `https://`, `data:`) are returned as-is.
/// - Relative URLs are joined with `base_url`.
/// - If no `base_url` is configured, the URL is returned as-is.
pub fn resolve_url(url: &str, options: &MarkdownRenderOptions) -> String {
  let base = match &options.image_base_url {
    Some(b) => b,
    None => return url.to_string(),
  };

  // Already absolute — don't touch.
  if url.starts_with("http://")
    || url.starts_with("https://")
    || url.starts_with("data:")
    || url.starts_with("//")
  {
    return url.to_string();
  }

  // Join base + relative. Ensure exactly one `/` between them.
  let base = base.trim_end_matches('/');
  let rel = url.trim_start_matches('/');
  format!("{base}/{rel}")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn resolve_relative_url_with_base() {
    let opts = MarkdownRenderOptions::default()
      .with_image_base_url("https://raw.githubusercontent.com/owner/repo/main");
    assert_eq!(
      resolve_url("images/logo.png", &opts),
      "https://raw.githubusercontent.com/owner/repo/main/images/logo.png"
    );
  }

  #[test]
  fn resolve_relative_url_with_leading_slash() {
    let opts = MarkdownRenderOptions::default().with_image_base_url("https://example.com/assets");
    assert_eq!(
      resolve_url("/img/banner.svg", &opts),
      "https://example.com/assets/img/banner.svg"
    );
  }

  #[test]
  fn resolve_relative_url_base_trailing_slash() {
    let opts = MarkdownRenderOptions::default().with_image_base_url("https://example.com/assets/");
    assert_eq!(
      resolve_url("icon.png", &opts),
      "https://example.com/assets/icon.png"
    );
  }

  #[test]
  fn absolute_url_unchanged() {
    let opts = MarkdownRenderOptions::default().with_image_base_url("https://example.com");
    assert_eq!(
      resolve_url("https://cdn.example.com/image.png", &opts),
      "https://cdn.example.com/image.png"
    );
  }

  #[test]
  fn http_url_unchanged() {
    let opts = MarkdownRenderOptions::default().with_image_base_url("https://example.com");
    assert_eq!(
      resolve_url("http://cdn.example.com/image.png", &opts),
      "http://cdn.example.com/image.png"
    );
  }

  #[test]
  fn data_uri_unchanged() {
    let opts = MarkdownRenderOptions::default().with_image_base_url("https://example.com");
    let data_url = "data:image/png;base64,iVBORw0KGgo=";
    assert_eq!(resolve_url(data_url, &opts), data_url);
  }

  #[test]
  fn protocol_relative_url_unchanged() {
    let opts = MarkdownRenderOptions::default().with_image_base_url("https://example.com");
    assert_eq!(
      resolve_url("//cdn.example.com/img.png", &opts),
      "//cdn.example.com/img.png"
    );
  }

  #[test]
  fn no_base_url_returns_as_is() {
    let opts = MarkdownRenderOptions::default();
    assert_eq!(resolve_url("images/logo.png", &opts), "images/logo.png");
  }

  #[test]
  fn no_double_slashes_on_join() {
    let opts = MarkdownRenderOptions::default().with_image_base_url("https://example.com/");
    assert_eq!(
      resolve_url("/path/img.png", &opts),
      "https://example.com/path/img.png"
    );
  }
}
