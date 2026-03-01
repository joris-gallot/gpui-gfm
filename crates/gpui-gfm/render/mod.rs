//! Markdown rendering — converts parsed blocks/inlines into GPUI elements.

pub mod blocks;
pub mod code_block;
pub mod inline;
pub mod table;

use std::sync::Arc;

use gpui::{AnyElement, App, Hsla, SharedString};

use crate::github::GithubIssueReferenceContext;
use crate::types::ParsedMarkdown;

/// A link click handler.
pub type LinkHandlerFn = dyn Fn(&str, &mut gpui::Window, &mut App) + Send + Sync;

/// What to do after a link click.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkAction {
  /// Open the link with the default handler.
  Open,
  /// The handler already dealt with the link.
  Handled,
}

/// Theme colors for markdown rendering.
///
/// Provide sensible defaults so rendering works out of the box without
/// depending on `gpui_component::ActiveTheme`.
#[derive(Clone, Debug)]
pub struct MarkdownTheme {
  pub foreground: Hsla,
  pub muted_foreground: Hsla,
  pub background: Hsla,
  pub code_background: Hsla,
  pub border: Hsla,
  pub link: Hsla,
  pub accent: Hsla,
}

impl Default for MarkdownTheme {
  fn default() -> Self {
    Self::dark()
  }
}

impl MarkdownTheme {
  /// A reasonable dark theme.
  pub fn dark() -> Self {
    Self {
      foreground: Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.93,
        a: 1.0,
      },
      muted_foreground: Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.6,
        a: 1.0,
      },
      background: Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.1,
        a: 1.0,
      },
      code_background: Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.15,
        a: 1.0,
      },
      border: Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.25,
        a: 1.0,
      },
      link: Hsla {
        h: 0.58,
        s: 0.8,
        l: 0.65,
        a: 1.0,
      },
      accent: Hsla {
        h: 0.58,
        s: 0.7,
        l: 0.5,
        a: 1.0,
      },
    }
  }

  /// A reasonable light theme.
  pub fn light() -> Self {
    Self {
      foreground: Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.13,
        a: 1.0,
      },
      muted_foreground: Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.45,
        a: 1.0,
      },
      background: Hsla {
        h: 0.0,
        s: 0.0,
        l: 1.0,
        a: 1.0,
      },
      code_background: Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.96,
        a: 1.0,
      },
      border: Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.85,
        a: 1.0,
      },
      link: Hsla {
        h: 0.58,
        s: 0.8,
        l: 0.45,
        a: 1.0,
      },
      accent: Hsla {
        h: 0.58,
        s: 0.7,
        l: 0.5,
        a: 1.0,
      },
    }
  }
}

/// Options for rendering markdown.
#[derive(Clone, Default)]
pub struct MarkdownRenderOptions {
  /// Link click handler.
  pub on_link: Option<Arc<LinkHandlerFn>>,
  /// Theme colors.
  pub theme: Option<MarkdownTheme>,
  /// GitHub issue reference context for auto-linking `#123`.
  pub github_issue_reference_context: Option<GithubIssueReferenceContext>,
  /// Whether code blocks render at full height (no scroll cap).
  pub expand_code_blocks: bool,
  /// Base URL for resolving relative image paths.
  pub image_base_url: Option<SharedString>,
}

impl MarkdownRenderOptions {
  pub fn with_on_link(mut self, handler: Arc<LinkHandlerFn>) -> Self {
    self.on_link = Some(handler);
    self
  }

  pub fn with_theme(mut self, theme: MarkdownTheme) -> Self {
    self.theme = Some(theme);
    self
  }

  pub fn with_expanded_code_blocks(mut self) -> Self {
    self.expand_code_blocks = true;
    self
  }

  pub fn with_image_base_url(mut self, url: impl Into<SharedString>) -> Self {
    self.image_base_url = Some(url.into());
    self
  }

  /// Get the theme, falling back to dark theme default.
  pub fn theme(&self) -> &MarkdownTheme {
    self.theme.as_ref().unwrap_or(&DEFAULT_DARK_THEME)
  }
}

static DEFAULT_DARK_THEME: std::sync::LazyLock<MarkdownTheme> =
  std::sync::LazyLock::new(MarkdownTheme::dark);

/// Render a markdown source string to a GPUI element.
pub fn render_markdown(source: &str, options: &MarkdownRenderOptions, cx: &App) -> AnyElement {
  let parsed = crate::parse::parse_markdown(source);
  render_parsed_markdown(&parsed, options, cx)
}

/// Render a pre-parsed markdown document to a GPUI element.
pub fn render_parsed_markdown(
  parsed: &ParsedMarkdown,
  options: &MarkdownRenderOptions,
  cx: &App,
) -> AnyElement {
  blocks::render_blocks(parsed.blocks(), options, 0, cx)
}
