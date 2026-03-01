//! Markdown rendering — converts parsed blocks/inlines into GPUI elements.

pub mod blocks;
pub mod code_block;
pub mod image;
pub mod inline;
pub mod table;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

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

/// Persistent state for `<details>` toggle (open/close).
///
/// Shared via `Arc` so that click callbacks can mutate it.
#[derive(Clone, Default)]
pub struct DetailsState {
  /// Maps a details block ID → open/closed.
  open_map: Arc<Mutex<HashMap<usize, bool>>>,
  /// Counter for assigning unique IDs to details blocks during rendering.
  counter: Arc<AtomicUsize>,
}

impl DetailsState {
  /// Reset the counter before a new render pass (so IDs are stable).
  pub fn reset_counter(&self) {
    self.counter.store(0, Ordering::Relaxed);
  }

  /// Get the next details block ID.
  pub fn next_id(&self) -> usize {
    self.counter.fetch_add(1, Ordering::Relaxed)
  }

  /// Check if a details block is open, defaulting to `default_open`.
  pub fn is_open(&self, id: usize, default_open: bool) -> bool {
    let mut map = self.open_map.lock().unwrap();
    *map.entry(id).or_insert(default_open)
  }

  /// Toggle a details block's state, returning the new state.
  pub fn toggle(&self, id: usize, default_open: bool) -> bool {
    let mut map = self.open_map.lock().unwrap();
    let current = *map.entry(id).or_insert(default_open);
    let next = !current;
    map.insert(id, next);
    next
  }
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
  /// Font family for code blocks and inline code.
  pub code_font_family: SharedString,
  /// Whether this is a dark theme (used for dark/light image URL selection).
  pub is_dark: bool,
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
      code_font_family: "Menlo".into(),
      is_dark: true,
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
      code_font_family: "Menlo".into(),
      is_dark: false,
    }
  }
}

/// A function that provides a custom [`gpui::ImageSource`] for a resolved URL.
///
/// When set on [`MarkdownRenderOptions`], this is called instead of using
/// `gpui::img(url)` directly. Useful for auth headers, custom caching, etc.
pub type ImageLoaderFn = dyn Fn(&str) -> gpui::ImageSource + Send + Sync;

/// Options for rendering markdown.
#[derive(Clone, Default)]
pub struct MarkdownRenderOptions {
  /// Link click handler.
  pub on_link: Option<Arc<LinkHandlerFn>>,
  /// Theme colors.
  pub theme: Option<MarkdownTheme>,
  /// Whether code blocks render at full height (no scroll cap).
  pub expand_code_blocks: bool,
  /// Base URL for resolving relative image paths.
  pub image_base_url: Option<SharedString>,
  /// Context for auto-linking GitHub issue references (`#123`).
  ///
  /// When set, bare `#123` patterns in text are converted to clickable links
  /// pointing to `https://github.com/{owner}/{repo}/issues/{num}`.
  pub github_issue_reference_context: Option<GithubIssueReferenceContext>,
  /// Custom image source provider.
  ///
  /// When set, this function is called with the resolved image URL to produce
  /// a [`gpui::ImageSource`]. When `None`, `gpui::img(url)` is used directly
  /// (which requires an HTTP client registered on the GPUI `App`).
  pub image_loader: Option<Arc<ImageLoaderFn>>,
  /// Persistent state for `<details>` toggle.
  ///
  /// Created automatically on first use. Persists across re-renders so
  /// toggle state is maintained.
  pub details_state: DetailsState,
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

  pub fn with_github_issue_context(
    mut self,
    owner: impl Into<Arc<str>>,
    repo: impl Into<Arc<str>>,
  ) -> Self {
    self.github_issue_reference_context = Some(GithubIssueReferenceContext {
      owner: owner.into(),
      repo: repo.into(),
    });
    self
  }

  pub fn with_image_loader(mut self, loader: Arc<ImageLoaderFn>) -> Self {
    self.image_loader = Some(loader);
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
  // Reset the details ID counter so IDs are stable across re-renders.
  options.details_state.reset_counter();
  blocks::render_blocks(parsed.blocks(), options, 0, cx)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn details_state_default_closed() {
    let state = DetailsState::default();
    // Default to false when not explicitly set
    assert!(!state.is_open(0, false));
  }

  #[test]
  fn details_state_default_open() {
    let state = DetailsState::default();
    // Respects default_open = true
    assert!(state.is_open(0, true));
  }

  #[test]
  fn details_state_toggle() {
    let state = DetailsState::default();
    // Initially closed (default_open = false)
    assert!(!state.is_open(0, false));
    // Toggle → open
    state.toggle(0, false);
    assert!(state.is_open(0, false));
    // Toggle → closed again
    state.toggle(0, false);
    assert!(!state.is_open(0, false));
  }

  #[test]
  fn details_state_toggle_from_open() {
    let state = DetailsState::default();
    // Initially open (default_open = true)
    assert!(state.is_open(0, true));
    // Toggle → closed
    state.toggle(0, true);
    assert!(!state.is_open(0, true));
  }

  #[test]
  fn details_state_independent_ids() {
    let state = DetailsState::default();
    state.toggle(0, false); // id 0 → open
    // id 1 should still be at default (closed)
    assert!(state.is_open(0, false));
    assert!(!state.is_open(1, false));
  }

  #[test]
  fn details_state_counter_increments() {
    let state = DetailsState::default();
    assert_eq!(state.next_id(), 0);
    assert_eq!(state.next_id(), 1);
    assert_eq!(state.next_id(), 2);
  }

  #[test]
  fn details_state_counter_resets() {
    let state = DetailsState::default();
    assert_eq!(state.next_id(), 0);
    assert_eq!(state.next_id(), 1);
    state.reset_counter();
    assert_eq!(state.next_id(), 0);
  }

  #[test]
  fn details_state_persists_across_resets() {
    let state = DetailsState::default();
    let id = state.next_id();
    state.toggle(id, false); // open
    assert!(state.is_open(id, false));
    // Reset counter (simulating a re-render)
    state.reset_counter();
    // State should persist
    assert!(state.is_open(0, false));
  }
}
