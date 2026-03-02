# gpui-gfm

GitHub Flavored Markdown renderer for [GPUI](https://gpui.rs), powered by [comrak](https://github.com/kivikakk/comrak).

## Usage

```rust
use gpui_gfm::{render_markdown, MarkdownRenderOptions, MarkdownTheme};

let options = MarkdownRenderOptions::default()
    .with_theme(MarkdownTheme::dark())
    .with_on_link(Arc::new(|url, _window, _cx| {
        println!("clicked: {url}");
    }))
    .with_expanded_code_blocks()
    .with_indentation_dots()
    .with_image_base_url("https://example.com/")
    .with_github_issue_context("owner", "repo");

let element = render_markdown("# Hello **world**", &options, cx);
```

### Caching

Use `MarkdownCache` to avoid re-parsing unchanged sources:

```rust
use gpui_gfm::MarkdownCache;

let cache = MarkdownCache::default(); // max 256 entries
let parsed = cache.get_or_parse(source);
let element = render_parsed_markdown(&parsed, &options, cx);
```

## GFM Support

Based on the [GitHub Flavored Markdown Spec](https://github.github.com/gfm/):

| Feature                         | Spec                                                                    | Status           |
| ------------------------------- | ----------------------------------------------------------------------- | ---------------- |
| ATX headings `#` … `######`     | [§4.2](https://github.github.com/gfm/#atx-headings)                     | ✅               |
| Setext headings                 | [§4.3](https://github.github.com/gfm/#setext-headings)                  | ✅               |
| Fenced code blocks              | [§4.5](https://github.github.com/gfm/#fenced-code-blocks)               | ✅               |
| Indented code blocks            | [§4.4](https://github.github.com/gfm/#indented-code-blocks)             | ✅               |
| Paragraphs                      | [§4.8](https://github.github.com/gfm/#paragraphs)                       | ✅               |
| Block quotes                    | [§5.1](https://github.github.com/gfm/#block-quotes)                     | ✅               |
| Ordered / unordered lists       | [§5.2–5.3](https://github.github.com/gfm/#lists)                        | ✅               |
| Thematic breaks `---`           | [§4.1](https://github.github.com/gfm/#thematic-breaks)                  | ✅               |
| Tables (GFM extension)          | [§4.10](https://github.github.com/gfm/#tables-extension-)               | ✅               |
| Task list items (GFM extension) | [§5.4](https://github.github.com/gfm/#task-list-items-extension-)       | ✅               |
| Strikethrough (GFM extension)   | [§6.5](https://github.github.com/gfm/#strikethrough-extension-)         | ✅               |
| Autolinks (GFM extension)       | [§6.9](https://github.github.com/gfm/#autolinks-extension-)             | ✅               |
| Bold / italic emphasis          | [§6.2–6.4](https://github.github.com/gfm/#emphasis-and-strong-emphasis) | ✅               |
| Inline code                     | [§6.1](https://github.github.com/gfm/#code-spans)                       | ✅               |
| Links                           | [§6.6](https://github.github.com/gfm/#links)                            | ✅               |
| Images                          | [§6.7](https://github.github.com/gfm/#images)                           | ✅ (placeholder) |
| Hard / soft line breaks         | [§6.11–6.12](https://github.github.com/gfm/#hard-line-breaks)           | ✅               |
| HTML blocks / inline HTML       | [§4.6, §6.8](https://github.github.com/gfm/#html-blocks)                | ✅               |
| `<details><summary>`            | HTML block                                                              | ✅               |
| Syntax highlighting             | —                                                                       | 🔜               |
| Footnotes                       | —                                                                       | 🔜               |

## Render Options

| Option                           | Type                              | Description                                                         |
| -------------------------------- | --------------------------------- | ------------------------------------------------------------------- |
| `theme`                          | `MarkdownTheme`                   | Color palette and fonts — `dark()` or `light()` (default: dark)     |
| `on_link`                        | `Fn(&str, &mut Window, &mut App)` | Custom link click handler                                           |
| `expand_code_blocks`             | `bool`                            | Render code blocks at full height, no scroll cap (default: `false`) |
| `show_indentation_dots`          | `bool`                            | Show faint dots on leading spaces in code blocks (default: `false`) |
| `image_base_url`                 | `SharedString`                    | Base URL for resolving relative image paths                         |
| `image_loader`                   | `Fn(&str) -> ImageSource`         | Custom image source provider (auth headers, caching, etc.)          |
| `github_issue_reference_context` | `{owner, repo}`                   | Auto-link `#123` patterns to GitHub issues                          |
| `github_code_reference_previews` | `HashMap<url, preview>`           | Replace GitHub blob URLs with inline code preview cards             |
| `details_state`                  | `DetailsState`                    | Persistent open/close state for `<details>` blocks                  |
| `overrides`                      | `RenderOverrides`                 | Replace default renderers for individual block types                |

### Render Overrides

Each field in `RenderOverrides` lets you replace the built-in renderer for a block type:

| Override         | Signature                                                    |
| ---------------- | ------------------------------------------------------------ |
| `paragraph`      | `Fn(AnyElement, &App) -> AnyElement`                         |
| `heading`        | `Fn(u8, AnyElement, &App) -> AnyElement`                     |
| `code_block`     | `Fn(&CodeBlock, &MarkdownRenderOptions, &App) -> AnyElement` |
| `list`           | `Fn(&List, &MarkdownRenderOptions, &App) -> AnyElement`      |
| `list_item`      | `Fn(ListItemView, &App) -> AnyElement`                       |
| `block_quote`    | `Fn(AnyElement, &App) -> AnyElement`                         |
| `thematic_break` | `Fn(&App) -> AnyElement`                                     |
| `table`          | `Fn(&Table, &MarkdownRenderOptions, &App) -> AnyElement`     |

## Playground

```sh
cargo run -p playground
```

`Cmd+Enter` to render, `Cmd+Q` to quit.
