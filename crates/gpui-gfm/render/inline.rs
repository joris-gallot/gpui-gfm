//! Inline text rendering — converts `Vec<Inline>` into styled GPUI text.

use gpui::{
  AnyElement, App, Font, FontStyle, FontWeight, MouseButton, SharedString, StrikethroughStyle,
  StyledText, TextRun, UnderlineStyle, div, prelude::*, px,
};

use crate::types::*;

use super::MarkdownRenderOptions;

/// A segment of inline content — either plain text or a clickable link.
#[derive(Debug)]
enum InlineSegment {
  /// Plain (non-link) text with its runs.
  Text { text: String, runs: Vec<TextRun> },
  /// A link segment: styled text + the URL.
  Link {
    text: String,
    runs: Vec<TextRun>,
    url: String,
  },
}

/// Render a list of inlines as GPUI elements.
///
/// When `on_link` is set, link segments become clickable with cursor pointer.
/// Otherwise everything is a single `StyledText`.
pub fn render_inline_text(
  inlines: &[Inline],
  options: &MarkdownRenderOptions,
  cx: &App,
) -> AnyElement {
  // Expand GitHub issue references (#123 → links) if configured.
  let expanded;
  let inlines = if let Some(gh) = &options.github_issue_reference_context {
    expanded = crate::github::expand_issue_references(inlines, gh);
    &expanded
  } else {
    inlines
  };

  if options.on_link.is_some() {
    render_inline_segmented(inlines, options, cx)
  } else {
    render_inline_flat(inlines, options, cx)
  }
}

/// Fast path: no link handler → single StyledText (no segmentation needed).
fn render_inline_flat(inlines: &[Inline], options: &MarkdownRenderOptions, cx: &App) -> AnyElement {
  let mut text = String::new();
  let mut runs: Vec<TextRun> = Vec::new();

  flatten_inlines(
    inlines,
    &mut text,
    &mut runs,
    &InlineContext::default(),
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

/// Segmented path: split into text/link segments, wrap links in clickable divs.
fn render_inline_segmented(
  inlines: &[Inline],
  options: &MarkdownRenderOptions,
  cx: &App,
) -> AnyElement {
  let segments = collect_segments(inlines, options, cx);

  if segments.is_empty() {
    return div().into_any_element();
  }

  // If there's only one non-link segment, shortcut to StyledText.
  if segments.len() == 1 {
    if let InlineSegment::Text { text, runs } = &segments[0] {
      let shared: SharedString = text.clone().into();
      return StyledText::new(shared)
        .with_runs(runs.clone())
        .into_any_element();
    }
  }

  let on_link = options.on_link.clone();

  // Use a flex-wrap container so segments flow inline.
  let mut container = div().flex().flex_wrap().items_baseline();

  for (ix, segment) in segments.into_iter().enumerate() {
    match segment {
      InlineSegment::Text { text, runs } => {
        if !text.is_empty() {
          let shared: SharedString = text.into();
          container = container.child(StyledText::new(shared).with_runs(runs));
        }
      }
      InlineSegment::Link { text, runs, url } => {
        if !text.is_empty() {
          let shared: SharedString = text.into();
          let link_id: SharedString = format!("gfm-link-{ix}").into();
          let on_link = on_link.clone();
          let url_clone = url.clone();

          let styled = StyledText::new(shared).with_runs(runs);
          let mut link_div = div().id(link_id).cursor_pointer().child(styled);

          if let Some(handler) = on_link {
            link_div = link_div.on_mouse_down(MouseButton::Left, move |_, window, cx| {
              handler(&url_clone, window, cx);
            });
          } else {
            // Fallback: open URL with system handler
            let url_fallback = url;
            link_div = link_div.on_mouse_down(MouseButton::Left, move |_, _window, cx| {
              cx.open_url(&url_fallback);
            });
          }

          container = container.child(link_div);
        }
      }
    }
  }

  container.into_any_element()
}

/// Collect inline content into a list of text/link segments.
fn collect_segments(
  inlines: &[Inline],
  options: &MarkdownRenderOptions,
  cx: &App,
) -> Vec<InlineSegment> {
  let mut segments: Vec<InlineSegment> = Vec::new();
  collect_segments_inner(
    inlines,
    &mut segments,
    &InlineContext::default(),
    options,
    cx,
  );
  segments
}

/// Recursively walk inlines and push into the correct segment.
fn collect_segments_inner(
  inlines: &[Inline],
  segments: &mut Vec<InlineSegment>,
  ctx: &InlineContext,
  options: &MarkdownRenderOptions,
  cx: &App,
) {
  let theme = options.theme();

  for inline in inlines {
    match inline {
      Inline::Text(value) => {
        let run = make_text_run(0..value.len(), ctx, theme);
        push_to_current_segment(segments, ctx, value.clone(), run);
      }

      Inline::Code(value) => {
        let code_ctx = InlineContext { code: true, ..*ctx };
        let run = make_text_run(0..value.len(), &code_ctx, theme);
        push_to_current_segment(segments, ctx, value.clone(), run);
      }

      Inline::SoftBreak => {
        let run = make_text_run(0..1, ctx, theme);
        push_to_current_segment(segments, ctx, " ".to_string(), run);
      }

      Inline::HardBreak => {
        let run = make_text_run(0..1, ctx, theme);
        push_to_current_segment(segments, ctx, "\n".to_string(), run);
      }

      Inline::Strong(children) => {
        let bold_ctx = InlineContext { bold: true, ..*ctx };
        collect_segments_inner(children, segments, &bold_ctx, options, cx);
      }

      Inline::Emphasis(children) => {
        let italic_ctx = InlineContext {
          italic: true,
          ..*ctx
        };
        collect_segments_inner(children, segments, &italic_ctx, options, cx);
      }

      Inline::Strikethrough(children) => {
        let strike_ctx = InlineContext {
          strikethrough: true,
          ..*ctx
        };
        collect_segments_inner(children, segments, &strike_ctx, options, cx);
      }

      Inline::Link { url, content, .. } => {
        // Start a new Link segment.
        let resolved = resolve_url(url, options);
        segments.push(InlineSegment::Link {
          text: String::new(),
          runs: Vec::new(),
          url: resolved,
        });
        let link_ctx = InlineContext { link: true, ..*ctx };
        collect_segments_inner(content, segments, &link_ctx, options, cx);
        // After the link content, force a new Text segment for subsequent content.
        segments.push(InlineSegment::Text {
          text: String::new(),
          runs: Vec::new(),
        });
      }

      Inline::Image { alt, url, .. } => {
        let _resolved_url = resolve_url(url, options);
        if !alt.is_empty() {
          let run = make_text_run(0..alt.len(), ctx, theme);
          push_to_current_segment(segments, ctx, alt.clone(), run);
        }
      }
    }
  }
}

/// Append text+run to the current (last) segment, creating one if needed.
fn push_to_current_segment(
  segments: &mut Vec<InlineSegment>,
  ctx: &InlineContext,
  value: String,
  mut run: TextRun,
) {
  // If we're inside a link, append to the current Link segment.
  // Otherwise append to the current Text segment (or create one).
  let needs_new = segments.is_empty()
    || match segments.last() {
      Some(InlineSegment::Link { .. }) => !ctx.link,
      Some(InlineSegment::Text { .. }) => ctx.link,
      None => true,
    };

  if needs_new {
    if ctx.link {
      segments.push(InlineSegment::Link {
        text: String::new(),
        runs: Vec::new(),
        url: String::new(),
      });
    } else {
      segments.push(InlineSegment::Text {
        text: String::new(),
        runs: Vec::new(),
      });
    }
  }

  let seg = segments.last_mut().unwrap();
  match seg {
    InlineSegment::Text { text, runs } | InlineSegment::Link { text, runs, .. } => {
      // Fix run offset: run.len is correct, but we need to adjust to segment-local position.
      run.len = value.len();
      text.push_str(&value);
      runs.push(run);
    }
  }
}

/// Context tracking the current inline formatting state.
#[derive(Clone, Copy)]
struct InlineContext {
  bold: bool,
  italic: bool,
  strikethrough: bool,
  code: bool,
  link: bool,
}

impl Default for InlineContext {
  fn default() -> Self {
    Self {
      bold: false,
      italic: false,
      strikethrough: false,
      code: false,
      link: false,
    }
  }
}

/// Recursively flatten inlines into a plain text string + TextRun spans.
/// Used by the fast (non-segmented) path.
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
        let _resolved_url = resolve_url(url, options);
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

  // --- Segmentation tests (pure logic, no App context needed) ---

  /// Helper: run collect_segments_inner directly without App.
  fn test_segments(inlines: &[Inline], opts: &MarkdownRenderOptions) -> Vec<InlineSegment> {
    let mut segments: Vec<InlineSegment> = Vec::new();
    // We call collect_segments_inner with a dummy — it only uses options.theme()
    // and resolve_url() which don't need App. We pass through the cx but the
    // flatten functions don't actually use it.
    // Instead, we test the pure segmentation logic via push_to_current_segment.
    let ctx = InlineContext::default();

    fn walk(
      inlines: &[Inline],
      segments: &mut Vec<InlineSegment>,
      ctx: &InlineContext,
      opts: &MarkdownRenderOptions,
    ) {
      let theme = opts.theme();
      for inline in inlines {
        match inline {
          Inline::Text(value) => {
            let run = make_text_run(0..value.len(), ctx, theme);
            push_to_current_segment(segments, ctx, value.clone(), run);
          }
          Inline::Code(value) => {
            let code_ctx = InlineContext { code: true, ..*ctx };
            let run = make_text_run(0..value.len(), &code_ctx, theme);
            push_to_current_segment(segments, ctx, value.clone(), run);
          }
          Inline::Strong(children) => {
            let bold_ctx = InlineContext { bold: true, ..*ctx };
            walk(children, segments, &bold_ctx, opts);
          }
          Inline::Emphasis(children) => {
            let italic_ctx = InlineContext {
              italic: true,
              ..*ctx
            };
            walk(children, segments, &italic_ctx, opts);
          }
          Inline::Link { url, content, .. } => {
            let resolved = resolve_url(url, opts);
            segments.push(InlineSegment::Link {
              text: String::new(),
              runs: Vec::new(),
              url: resolved,
            });
            let link_ctx = InlineContext { link: true, ..*ctx };
            walk(content, segments, &link_ctx, opts);
            segments.push(InlineSegment::Text {
              text: String::new(),
              runs: Vec::new(),
            });
          }
          Inline::SoftBreak => {
            let run = make_text_run(0..1, ctx, theme);
            push_to_current_segment(segments, ctx, " ".to_string(), run);
          }
          _ => {}
        }
      }
    }

    walk(inlines, &mut segments, &ctx, opts);
    segments
  }

  fn non_empty(segments: &[InlineSegment]) -> Vec<&InlineSegment> {
    segments
      .iter()
      .filter(|s| match s {
        InlineSegment::Text { text, .. } => !text.is_empty(),
        InlineSegment::Link { text, .. } => !text.is_empty(),
      })
      .collect()
  }

  #[test]
  fn segments_plain_text_only() {
    let inlines = vec![Inline::Text("Hello world".into())];
    let opts = MarkdownRenderOptions::default();
    let segments = test_segments(&inlines, &opts);
    let ne = non_empty(&segments);
    assert_eq!(ne.len(), 1);
    assert!(matches!(ne[0], InlineSegment::Text { text, .. } if text == "Hello world"));
  }

  #[test]
  fn segments_link_produces_three_segments() {
    let inlines = vec![
      Inline::Text("before ".into()),
      Inline::Link {
        url: "https://example.com".into(),
        title: None,
        content: vec![Inline::Text("click".into())],
      },
      Inline::Text(" after".into()),
    ];
    let opts = MarkdownRenderOptions::default();
    let segments = test_segments(&inlines, &opts);
    let ne = non_empty(&segments);

    assert_eq!(ne.len(), 3);
    assert!(matches!(ne[0], InlineSegment::Text { text, .. } if text == "before "));
    assert!(
      matches!(ne[1], InlineSegment::Link { text, url, .. } if text == "click" && url == "https://example.com")
    );
    assert!(matches!(ne[2], InlineSegment::Text { text, .. } if text == " after"));
  }

  #[test]
  fn segments_link_with_bold_content() {
    let inlines = vec![Inline::Link {
      url: "https://example.com".into(),
      title: None,
      content: vec![Inline::Strong(vec![Inline::Text("bold link".into())])],
    }];
    let opts = MarkdownRenderOptions::default();
    let segments = test_segments(&inlines, &opts);
    let ne = non_empty(&segments);

    assert_eq!(ne.len(), 1);
    assert!(
      matches!(ne[0], InlineSegment::Link { text, url, .. } if text == "bold link" && url == "https://example.com")
    );
  }

  #[test]
  fn segments_multiple_links() {
    let inlines = vec![
      Inline::Link {
        url: "https://a.com".into(),
        title: None,
        content: vec![Inline::Text("A".into())],
      },
      Inline::Text(" and ".into()),
      Inline::Link {
        url: "https://b.com".into(),
        title: None,
        content: vec![Inline::Text("B".into())],
      },
    ];
    let opts = MarkdownRenderOptions::default();
    let segments = test_segments(&inlines, &opts);
    let ne = non_empty(&segments);

    assert_eq!(ne.len(), 3);
    assert!(
      matches!(ne[0], InlineSegment::Link { text, url, .. } if text == "A" && url == "https://a.com")
    );
    assert!(matches!(ne[1], InlineSegment::Text { text, .. } if text == " and "));
    assert!(
      matches!(ne[2], InlineSegment::Link { text, url, .. } if text == "B" && url == "https://b.com")
    );
  }

  #[test]
  fn segments_link_url_resolved() {
    let inlines = vec![Inline::Link {
      url: "page.html".into(),
      title: None,
      content: vec![Inline::Text("link".into())],
    }];
    let opts = MarkdownRenderOptions::default().with_image_base_url("https://example.com/docs");
    let segments = test_segments(&inlines, &opts);

    let link = segments
      .iter()
      .find(|s| matches!(s, InlineSegment::Link { text, .. } if !text.is_empty()));
    assert!(
      matches!(link, Some(InlineSegment::Link { url, .. }) if url == "https://example.com/docs/page.html")
    );
  }
}
