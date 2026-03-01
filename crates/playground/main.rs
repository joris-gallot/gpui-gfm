mod input;

use gpui::{
  App, Application, Bounds, Context, Entity, FocusHandle, Focusable, KeyBinding, MouseButton,
  SharedString, Window, WindowBounds, WindowOptions, actions, div, prelude::*, px, size,
};
use gpui_gfm::render::{MarkdownRenderOptions, MarkdownTheme};
use input::*;
use std::sync::Arc;

actions!(playground, [Quit, RenderMarkdown]);

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

With `image_base_url` = `https://raw.githubusercontent.com/owner/repo/main`:

![Relative image](images/logo.png)
![Absolute image](https://cdn.example.com/badge.svg)

## Inline Code (§6.1)

Use `println!()` to print. Double backticks: ``code with `backtick` inside``.

## Fenced Code Blocks (§4.5)

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

## Render Options Demo

| Option | Value |
|--------|-------|
| `theme` | `MarkdownTheme::dark()` |
| `code_font_family` | Menlo (monospace) |
| `image_base_url` | `https://raw.githubusercontent.com/owner/repo/main` |
| `expand_code_blocks` | `false` (scroll cap at 400px) |
| `on_link` | Custom handler: logs URL to stdout |
| `details_state` | Shared state for toggle persistence |

---

*End of GFM feature demo.*
"#;

struct MarkdownPlayground {
  text_input: Entity<TextInput>,
  rendered_source: SharedString,
  options: MarkdownRenderOptions,
  focus_handle: FocusHandle,
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
}

impl Focusable for MarkdownPlayground {
  fn focus_handle(&self, _: &App) -> FocusHandle {
    self.focus_handle.clone()
  }
}

impl Render for MarkdownPlayground {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = self.options.theme();
    let rendered = gpui_gfm::render_markdown(&self.rendered_source, &self.options, cx);

    div()
      .key_context("Playground")
      .track_focus(&self.focus_handle(cx))
      .on_action(cx.listener(Self::render_markdown))
      .flex()
      .flex_col()
      .size_full()
      .bg(theme.background)
      .text_color(theme.foreground)
      // Toolbar
      .child(
        div()
          .flex()
          .items_center()
          .justify_between()
          .px_3()
          .py_2()
          .border_b_1()
          .border_color(theme.border)
          .child(
            div()
              .text_sm()
              .font_weight(gpui::FontWeight::BOLD)
              .child("gpui-gfm playground"),
          )
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
              .hover(|s| s.opacity(0.85))
              .on_mouse_down(MouseButton::Left, cx.listener(Self::on_render_click))
              .child("Render ⏎"),
          ),
      )
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
          cx.new(|cx| MarkdownPlayground {
            text_input,
            rendered_source: SAMPLE_MARKDOWN.into(),
            options: MarkdownRenderOptions {
              theme: Some(MarkdownTheme::dark()),
              image_base_url: Some("https://raw.githubusercontent.com/owner/repo/main".into()),
              on_link: Some(Arc::new(|url, _window, _cx| {
                println!("[on_link] clicked: {url}");
              })),
              ..Default::default()
            },
            focus_handle: cx.focus_handle(),
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
