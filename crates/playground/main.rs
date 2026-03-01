mod input;

use gpui::{
  App, Application, Bounds, Context, Entity, FocusHandle, Focusable, KeyBinding, MouseButton,
  SharedString, Window, WindowBounds, WindowOptions, actions, div, prelude::*, px, size,
};
use gpui_gfm::render::{MarkdownRenderOptions, MarkdownTheme};
use input::*;

actions!(playground, [Quit, RenderMarkdown]);

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
