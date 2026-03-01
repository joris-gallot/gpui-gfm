use gpui::{
  App, Application, Bounds, Context, SharedString, Window, WindowBounds, WindowOptions, div,
  prelude::*, px, size,
};
use gpui_gfm::render::{MarkdownRenderOptions, MarkdownTheme};

const SAMPLE_MARKDOWN: &str = r#"# gpui-gfm playground

A **GitHub Flavored Markdown** renderer for [GPUI](https://gpui.rs).

## Features

- [x] Headings
- [x] **Bold**, *italic*, ~~strikethrough~~
- [x] Links and inline `code`
- [ ] Images (coming soon)

## Code block

```rust
fn main() {
    println!("Hello, GFM!");
}
```

## Table

| Feature | Status |
|---------|--------|
| Parsing | ✅ Done |
| Rendering | ✅ Done |
| Syntax highlighting | 🔜 Planned |

## Blockquote

> This is a blockquote.
> It can span multiple lines.

---

*That's all for now!*
"#;

struct MarkdownPlayground {
  source: SharedString,
  options: MarkdownRenderOptions,
}

impl Render for MarkdownPlayground {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let rendered = gpui_gfm::render_markdown(&self.source, &self.options, cx);
    div()
      .id("playground-root")
      .flex()
      .flex_col()
      .size_full()
      .overflow_y_scroll()
      .bg(self.options.theme().background)
      .p_4()
      .child(rendered)
  }
}

fn main() {
  Application::with_platform(gpui_platform::current_platform(false)).run(|cx: &mut App| {
    let bounds = Bounds::centered(None, size(px(800.), px(600.0)), cx);
    cx.open_window(
      WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        ..Default::default()
      },
      |_, cx| {
        cx.new(|_| MarkdownPlayground {
          source: SAMPLE_MARKDOWN.into(),
          options: MarkdownRenderOptions {
            theme: Some(MarkdownTheme::dark()),
            ..Default::default()
          },
        })
      },
    )
    .unwrap();
  });
}
