mod input;

use gpui::{
  App, Application, Bounds, Context, Entity, FocusHandle, Focusable, KeyBinding, MouseButton,
  SharedString, Window, WindowBounds, WindowOptions, actions, div, prelude::*, px, size,
};
use gpui_gfm::github::GithubIssueReferenceContext;
use gpui_gfm::render::{MarkdownRenderOptions, MarkdownTheme};
use input::*;
use std::sync::Arc;

actions!(playground, [Quit, RenderMarkdown, FetchReadme]);

const SAMPLE_MARKDOWN: &str = r#"# gpui-gfm playground

A **GitHub Flavored Markdown** renderer for [GPUI](https://gpui.rs).

## ATX Headings (§4.2)

# Heading 1
## Heading 2
### Heading 3
#### Heading 4
##### Heading 5
###### Heading 6

Setext Heading 1 (§4.3)
=======================

Setext Heading 2 (§4.3)
-----------------------

## Paragraphs (§4.8)

This is a paragraph with **bold**, *italic*, ~~strikethrough~~, and `inline code`.

This is another paragraph. Soft breaks
are rendered as spaces.

Hard break with two trailing spaces:  
This is on a new line.

## Emphasis (§6.2–6.4)

- **Bold text**
- *Italic text*
- ***Bold and italic***
- **~~Bold strikethrough~~**
- *~~Italic strikethrough~~*

## Strikethrough — GFM extension (§6.5)

~~This text is deleted.~~

## Links (§6.6)

- [Inline link](https://github.com)
- [Link with title](https://github.com "GitHub")
- <https://github.com> (autolink §6.9)
- **Bold** text, then [a link](https://example.com), then normal text.
- Mixed: *italic* [link1](https://a.com) middle [link2](https://b.com) end.

## Images (§6.7)

**Inline badge** (rendered as 18px-tall image): Check build: ![CI](https://github.com/zed-industries/zed/actions/workflows/ci.yml/badge.svg)

**Block image** (paragraph = single image → full width):

![Zed Editor](https://zed.dev/img/og-image.png)

Mixed text with inline image: status ![badge](https://img.shields.io/badge/build-passing-green) and more text.

With `image_base_url` = `https://raw.githubusercontent.com/owner/repo/main`:

![Relative image](images/logo.png)

## Inline Code (§6.1)

Use `println!()` to print. Double backticks: ``code with `backtick` inside``.

## Fenced Code Blocks (§4.5)

Hover over a code block to reveal the **Copy** button (top-right).

```rust
fn main() {
    let long = "This line is intentionally very long to demonstrate horizontal scrolling in code blocks — it should not wrap";
    println!("{long}");
}
```

```python
def fibonacci(n: int) -> int:
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)
```

## Indented Code Block (§4.4)

    This is an indented code block.
    It uses 4 spaces of indentation.
    No language label is shown.

## Block Quotes (§5.1)

> A blockquote with **bold** and *italic*.
>
> > Nested blockquote.

## Unordered List (§5.3)

- Item one
- Item two
  - Nested item
  - Another nested
- Item three

## Ordered List (§5.2)

1. First item
2. Second item
3. Third item
   1. Nested ordered
   2. More nested

## Task List — GFM extension (§5.4)

- [x] Completed task
- [x] Another done
- [ ] Pending task
- [ ] Still todo

## Table — GFM extension (§4.10)

| Feature | Status | Notes |
|:--------|:------:|------:|
| Parsing | ✅ | comrak-based |
| Rendering | ✅ | GPUI elements |
| Left align | ✅ | `:---` |
| Center align | ✅ | `:---:` |
| Right align | ✅ | `---:` |

## Thematic Break (§4.1)

Content above.

---

Content below.

## Details / Summary (HTML block)

<details>
<summary>Click to expand (closed by default)</summary>

Hidden content with **formatting** and `code`.

- Nested item A
- Nested item B

</details>

<details open>
<summary>Starts open</summary>

This section is visible because of the `open` attribute.

</details>

## HTML: `<div align="center">`

<div align="center">

Centered content via HTML align attribute.

</div>

## GitHub Issue Auto-linking

With `github_issue_reference_context` = `zed-industries/zed`:

- See #123 for the bug report.
- Fixed in #42, related to #99.
- In code: `#456` (not linked).
- In a word: foo#789 (not linked).
- HTML entity &#123; (not linked).
- Already a link: [#100](https://github.com/zed-industries/zed/issues/100) (not double-linked).
- **Bold #55** works too.

## Render Options Demo

| Option | Value |
|--------|-------|
| `theme` | `MarkdownTheme::dark()` (`is_dark: true`) |
| `code_font_family` | Menlo (monospace) |
| `image_base_url` | `https://raw.githubusercontent.com/owner/repo/main` |
| `image_loader` | `None` (uses gpui's built-in `img()` loader) |
| `expand_code_blocks` | `false` (scroll cap at 400px) |
| `on_link` | Custom handler: logs URL to stdout |
| `details_state` | Shared state for toggle persistence |
| `github_issue_reference_context` | `zed-industries/zed` (auto-links `#123`) |

---

*End of GFM feature demo.*
"#;

struct MarkdownPlayground {
  text_input: Entity<TextInput>,
  url_input: Entity<TextInput>,
  rendered_source: SharedString,
  options: MarkdownRenderOptions,
  focus_handle: FocusHandle,
  is_fetching: bool,
}

impl MarkdownPlayground {
  fn render_markdown(&mut self, _: &RenderMarkdown, _: &mut Window, cx: &mut Context<Self>) {
    let source = self.text_input.read(cx).text().to_string();
    self.rendered_source = source.into();
    cx.notify();
  }

  fn on_render_click(&mut self, _: &gpui::MouseDownEvent, _: &mut Window, cx: &mut Context<Self>) {
    let source = self.text_input.read(cx).text().to_string();
    self.rendered_source = source.into();
    cx.notify();
  }

  fn fetch_readme(&mut self, _: &FetchReadme, _window: &mut Window, cx: &mut Context<Self>) {
    let url_text = self.url_input.read(cx).text().to_string();
    let (owner, repo) = match parse_github_url(&url_text) {
      Some(pair) => pair,
      None => return,
    };

    self.is_fetching = true;
    cx.notify();

    let text_input = self.text_input.clone();

    let raw_url = format!("https://raw.githubusercontent.com/{owner}/{repo}/HEAD/README.md");
    let image_base = format!("https://raw.githubusercontent.com/{owner}/{repo}/HEAD");
    let image_base_for_thread = image_base.clone();
    let owner_clone = owner.clone();
    let repo_clone = repo.clone();

    cx.spawn(async move |this, cx| {
      let result = std::thread::spawn(move || {
        let body = ureq::get(&raw_url).call()?.body_mut().read_to_string()?;
        // Some READMEs redirect to another file, e.g. "packages/ai/README.md"
        let body = maybe_follow_readme_redirect(&image_base_for_thread, &body)?;
        Ok::<String, Box<dyn std::error::Error + Send + Sync>>(body)
      })
      .join()
      .unwrap();

      this
        .update(cx, |this, cx| {
          this.is_fetching = false;
          match result {
            Ok(markdown) => {
              text_input.update(cx, |input, cx| {
                input.set_content(markdown.clone(), cx);
              });
              this.rendered_source = markdown.into();
              this.options.image_base_url = Some(image_base.into());
              this.options.github_issue_reference_context = Some(GithubIssueReferenceContext {
                owner: owner_clone.into(),
                repo: repo_clone.into(),
              });
            }
            Err(e) => {
              let error_md = format!("# Error fetching README\n\n```\n{e}\n```");
              text_input.update(cx, |input, cx| {
                input.set_content(error_md.clone(), cx);
              });
              this.rendered_source = error_md.into();
            }
          }
          cx.notify();
        })
        .ok();
    })
    .detach();
  }

  fn render_toolbar(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
    let theme = self.options.theme();
    div()
      .flex()
      .items_center()
      .justify_between()
      .gap_3()
      .px_3()
      .py_2()
      .border_b_1()
      .border_color(theme.border)
      // Title
      .child(
        div()
          .text_sm()
          .font_weight(gpui::FontWeight::BOLD)
          .flex_shrink_0()
          .child("gpui-gfm playground"),
      )
      // URL input
      .child(
        div()
          .flex()
          .items_center()
          .gap_2()
          .child(
            div()
              .w(px(400.0))
              .flex()
              .items_center()
              .px_2()
              .h(px(26.0))
              .rounded_md()
              .border_1()
              .border_color(theme.border)
              .bg(theme.code_background)
              .text_sm()
              .overflow_x_hidden()
              .child(self.url_input.clone()),
          )
          .child(if self.is_fetching {
            div()
              .text_xs()
              .text_color(theme.muted_foreground)
              .flex_shrink_0()
              .child("Fetching…")
              .into_any_element()
          } else {
            div().into_any_element()
          }),
      )
      // Render button
      .child(
        div()
          .id("render-btn")
          .px_3()
          .py_1()
          .rounded_md()
          .bg(theme.accent)
          .text_sm()
          .text_color(gpui::white())
          .cursor_pointer()
          .flex_shrink_0()
          .hover(|s| s.opacity(0.85))
          .on_mouse_down(MouseButton::Left, cx.listener(Self::on_render_click))
          .child("Render ⏎"),
      )
      .into_any_element()
  }
}

/// Parse a GitHub URL like `https://github.com/owner/repo` into `(owner, repo)`.
fn parse_github_url(url: &str) -> Option<(String, String)> {
  let url = url.trim().trim_end_matches('/');
  // Support: https://github.com/owner/repo or github.com/owner/repo
  let path = url
    .strip_prefix("https://github.com/")
    .or_else(|| url.strip_prefix("http://github.com/"))
    .or_else(|| url.strip_prefix("github.com/"))?;
  let parts: Vec<&str> = path.splitn(3, '/').collect();
  if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
    Some((parts[0].to_string(), parts[1].to_string()))
  } else {
    None
  }
}

/// If the fetched README content looks like a redirect (a single relative path
/// to another `.md` file), fetch that file instead. Returns the final content.
fn maybe_follow_readme_redirect(
  base_url: &str,
  content: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
  let trimmed = content.trim();
  // Heuristic: single line, ends with .md (case-insensitive), no spaces except
  // maybe in the path, and looks like a relative path (not a full sentence).
  if trimmed.lines().count() <= 1
    && trimmed.to_ascii_lowercase().ends_with(".md")
    && !trimmed.starts_with('#')
    && !trimmed.starts_with('!')
    && !trimmed.starts_with('[')
    && !trimmed.starts_with('<')
    && trimmed.len() < 256
  {
    let redirect_path = trimmed.trim_start_matches('/');
    let redirect_url = format!("{base_url}/{redirect_path}");
    let body = ureq::get(&redirect_url)
      .call()?
      .body_mut()
      .read_to_string()?;
    Ok(body)
  } else {
    Ok(content.to_string())
  }
}

impl Focusable for MarkdownPlayground {
  fn focus_handle(&self, _: &App) -> FocusHandle {
    self.focus_handle.clone()
  }
}

impl Render for MarkdownPlayground {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let toolbar = self.render_toolbar(cx);
    let theme = self.options.theme();
    let rendered = gpui_gfm::render_markdown(&self.rendered_source, &self.options, cx);

    div()
      .key_context("Playground")
      .track_focus(&self.focus_handle(cx))
      .on_action(cx.listener(Self::render_markdown))
      .on_action(cx.listener(Self::fetch_readme))
      .flex()
      .flex_col()
      .size_full()
      .bg(theme.background)
      .text_color(theme.foreground)
      // Toolbar
      .child(toolbar)
      // Split pane: input left, render right
      .child(
        div()
          .flex()
          .flex_1()
          .min_h_0()
          // Left: input
          .child(
            div()
              .flex()
              .flex_col()
              .w(gpui::relative(0.5))
              .h_full()
              .border_r_1()
              .border_color(theme.border)
              .child(
                div()
                  .px_3()
                  .py_1()
                  .text_xs()
                  .text_color(theme.muted_foreground)
                  .border_b_1()
                  .border_color(theme.border)
                  .child("Markdown source"),
              )
              .child(
                div()
                  .id("input-scroll")
                  .flex_1()
                  .min_h_0()
                  .overflow_y_scroll()
                  .p_2()
                  .text_sm()
                  .child(self.text_input.clone()),
              ),
          )
          // Right: rendered output
          .child(
            div()
              .flex()
              .flex_col()
              .w(gpui::relative(0.5))
              .h_full()
              .child(
                div()
                  .px_3()
                  .py_1()
                  .text_xs()
                  .text_color(theme.muted_foreground)
                  .border_b_1()
                  .border_color(theme.border)
                  .child("Rendered output"),
              )
              .child(
                div()
                  .id("render-scroll")
                  .flex_1()
                  .min_h_0()
                  .overflow_y_scroll()
                  .p_4()
                  .child(rendered),
              ),
          ),
      )
  }
}

fn main() {
  Application::with_platform(gpui_platform::current_platform(false)).run(|cx: &mut App| {
    cx.bind_keys([
      KeyBinding::new("backspace", Backspace, Some("TextInput")),
      KeyBinding::new("delete", Delete, Some("TextInput")),
      KeyBinding::new("left", Left, Some("TextInput")),
      KeyBinding::new("right", Right, Some("TextInput")),
      KeyBinding::new("up", Up, Some("TextInput")),
      KeyBinding::new("down", Down, Some("TextInput")),
      KeyBinding::new("shift-left", SelectLeft, Some("TextInput")),
      KeyBinding::new("shift-right", SelectRight, Some("TextInput")),
      KeyBinding::new("cmd-a", SelectAll, Some("TextInput")),
      KeyBinding::new("cmd-v", Paste, Some("TextInput")),
      KeyBinding::new("cmd-c", Copy, Some("TextInput")),
      KeyBinding::new("cmd-x", Cut, Some("TextInput")),
      KeyBinding::new("home", Home, Some("TextInput")),
      KeyBinding::new("end", End, Some("TextInput")),
      KeyBinding::new("enter", Enter, Some("TextInput")),
      KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, Some("TextInput")),
      KeyBinding::new("cmd-enter", RenderMarkdown, Some("Playground")),
      KeyBinding::new("cmd-q", Quit, None),
    ]);

    let bounds = Bounds::centered(None, size(px(1200.), px(700.0)), cx);
    let window = cx
      .open_window(
        WindowOptions {
          window_bounds: Some(WindowBounds::Windowed(bounds)),
          ..Default::default()
        },
        |_, cx| {
          let text_input = cx.new(|cx| TextInput::new(cx, SAMPLE_MARKDOWN.to_string()));
          let url_input = cx.new(|cx| {
            TextInput::new(cx, "https://github.com/zed-industries/zed".to_string()).on_enter(
              |window, cx| {
                window.dispatch_action(Box::new(FetchReadme), cx);
              },
            )
          });
          cx.new(|cx| MarkdownPlayground {
            text_input,
            url_input,
            rendered_source: SAMPLE_MARKDOWN.into(),
            options: MarkdownRenderOptions {
              theme: Some(MarkdownTheme::dark()),
              image_base_url: Some("https://raw.githubusercontent.com/owner/repo/main".into()),
              on_link: Some(Arc::new(|url, _window, _cx| {
                println!("[on_link] clicked: {url}");
              })),
              github_issue_reference_context: Some(GithubIssueReferenceContext {
                owner: "zed-industries".into(),
                repo: "zed".into(),
              }),
              ..Default::default()
            },
            focus_handle: cx.focus_handle(),
            is_fetching: false,
          })
        },
      )
      .unwrap();

    window
      .update(cx, |view, window, cx| {
        let handle = view.text_input.read(cx).focus_handle.clone();
        window.focus(&handle, cx);
        cx.activate(true);
      })
      .unwrap();

    cx.on_action(|_: &Quit, cx| cx.quit());
  });
}
