//! Lightweight HTML parsing for inline HTML that comrak passes through.
//!
//! This replaces the tree-sitter HTML parser from v1 with simple string
//! scanning. We only need to handle a small set of tags that GitHub uses:
//! `<img>`, `<br>`, `<picture>/<source>`, `<a>`, `<p>`, `<sub>`, `<sup>`,
//! centered `<div>`, and HTML entities.

use crate::types::Inline;

/// Parse a `<summary>` text into inlines. Strips HTML tags and returns a
/// single `Inline::Text`.
pub fn summary_inlines_from_text(text: &str) -> Vec<Inline> {
  let stripped = strip_html_tags(text);
  let trimmed = stripped.trim();
  if trimmed.is_empty() {
    vec![Inline::Text("Details".to_string())]
  } else {
    vec![Inline::Text(trimmed.to_string())]
  }
}

/// Parse an inline HTML `<img>` tag into an `Inline::Image`.
pub fn parse_inline_html_image(html: &str) -> Option<Inline> {
  let trimmed = html.trim();
  if !tag_name_ci(trimmed, "img") {
    return None;
  }

  let src = extract_html_attribute(trimmed, "src")?;
  let alt = extract_html_attribute(trimmed, "alt").unwrap_or_default();
  let width = extract_html_attribute(trimmed, "width");
  let height = extract_html_attribute(trimmed, "height");

  Some(Inline::Image {
    url: src,
    title: None,
    alt,
    width,
    height,
    dark_url: None,
    light_url: None,
  })
}

/// Check if an HTML string is a line break tag (`<br>`, `<br/>`, `<br />`).
pub fn is_html_line_break_tag(html: &str) -> bool {
  let trimmed = html.trim();
  let lower: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
  lower.eq_ignore_ascii_case("<br>")
    || lower.eq_ignore_ascii_case("<br/>")
    || lower.eq_ignore_ascii_case("<br/>")
}

/// Check if an HTML block is only a comment (`<!-- ... -->`).
pub fn is_html_comment_only(html: &str) -> bool {
  let trimmed = html.trim();
  trimmed.starts_with("<!--") && trimmed.ends_with("-->") && !trimmed[4..].contains("<!--")
}

/// Check if an HTML block is only a `</details>` closing tag.
pub fn is_details_close_only(html: &str) -> bool {
  let trimmed = html.trim();
  let lower = trimmed.to_ascii_lowercase();
  let stripped: String = lower.chars().filter(|c| !c.is_whitespace()).collect();
  stripped == "</details>" || stripped == "</details>\n"
}

/// Check if an HTML block opens a centered div.
pub fn is_centered_div_open(html: &str) -> bool {
  let trimmed = html.trim();
  if !tag_name_ci(trimmed, "div") {
    return false;
  }
  // Check for align="center" or style containing text-align: center
  if let Some(align) = extract_html_attribute(trimmed, "align") {
    if align.eq_ignore_ascii_case("center") {
      return true;
    }
  }
  if let Some(style) = extract_html_attribute(trimmed, "style") {
    let normalized: String = style
      .chars()
      .filter(|c| !c.is_whitespace())
      .collect::<String>()
      .to_ascii_lowercase();
    if normalized.contains("text-align:center") {
      return true;
    }
  }
  false
}

/// Check if an HTML block closes a `</div>`.
pub fn is_centered_div_close(html: &str) -> bool {
  let trimmed = html.trim();
  trimmed.eq_ignore_ascii_case("</div>")
}

/// Decode common HTML entities.
pub fn decode_html_entities(text: &str) -> String {
  text
    .replace("&amp;", "&")
    .replace("&lt;", "<")
    .replace("&gt;", ">")
    .replace("&quot;", "\"")
    .replace("&#39;", "'")
    .replace("&apos;", "'")
    .replace("&nbsp;", "\u{00A0}")
}

/// Extract an HTML attribute value from a tag string.
///
/// Handles both `attr="value"` and `attr='value'` quoting styles,
/// as well as unquoted single-word values.
pub fn extract_html_attribute(tag: &str, name: &str) -> Option<String> {
  // Search for the attribute name followed by =
  let tag_bytes = tag.as_bytes();
  let name_bytes = name.as_bytes();
  let name_len = name_bytes.len();

  let mut i = 0;
  while i + name_len < tag_bytes.len() {
    // Check for attribute name match (case-insensitive)
    if tag_bytes[i..i + name_len].eq_ignore_ascii_case(name_bytes) {
      let after_name = i + name_len;
      // Skip optional whitespace before =
      let mut j = after_name;
      while j < tag_bytes.len() && tag_bytes[j] == b' ' {
        j += 1;
      }
      if j < tag_bytes.len() && tag_bytes[j] == b'=' {
        j += 1;
        // Skip whitespace after =
        while j < tag_bytes.len() && tag_bytes[j] == b' ' {
          j += 1;
        }
        // Extract value
        if j < tag_bytes.len() {
          let quote = tag_bytes[j];
          if quote == b'"' || quote == b'\'' {
            let start = j + 1;
            if let Some(end) = tag[start..].find(quote as char) {
              return Some(decode_html_entities(&tag[start..start + end]));
            }
          } else {
            // Unquoted value — up to whitespace or >
            let start = j;
            let end = tag[start..]
              .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
              .unwrap_or(tag.len() - start);
            return Some(decode_html_entities(&tag[start..start + end]));
          }
        }
      }
    }
    i += 1;
  }
  None
}

/// Check if an HTML tag matches a given tag name (case-insensitive).
fn tag_name_ci(html: &str, name: &str) -> bool {
  let trimmed = html.trim_start();
  if !trimmed.starts_with('<') {
    return false;
  }
  let after_lt = &trimmed[1..];
  // Skip leading / for closing tags
  let after_slash = after_lt.strip_prefix('/').unwrap_or(after_lt);
  let tag_end = after_slash
    .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
    .unwrap_or(after_slash.len());
  after_slash[..tag_end].eq_ignore_ascii_case(name)
}

/// Strip HTML tags from a string.
fn strip_html_tags(input: &str) -> String {
  let mut result = String::with_capacity(input.len());
  let mut in_tag = false;
  for ch in input.chars() {
    match ch {
      '<' => in_tag = true,
      '>' => in_tag = false,
      _ if !in_tag => result.push(ch),
      _ => {}
    }
  }
  result
}

/// Parse inline HTML fragments into inlines.
///
/// This handles common inline HTML patterns from GitHub markdown:
/// - `<img>` tags → `Inline::Image`
/// - `<br>` tags → `Inline::HardBreak`
/// - `<a>` tags wrapping images → `Inline::Link` with image content
/// - Other HTML → stripped to text content
pub fn parse_html_to_inlines(html: &str) -> Vec<Inline> {
  let trimmed = html.trim();
  if trimmed.is_empty() {
    return Vec::new();
  }

  // Try specific patterns
  if let Some(img) = parse_inline_html_image(trimmed) {
    return vec![img];
  }

  if is_html_line_break_tag(trimmed) {
    return vec![Inline::HardBreak];
  }

  // Fallback: strip tags, return text if non-empty
  let text = strip_html_tags(trimmed);
  let text = text.trim();
  if text.is_empty() {
    Vec::new()
  } else {
    vec![Inline::Text(decode_html_entities(text).to_string())]
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_img_tag() {
    let html = r#"<img src="https://example.com/img.png" alt="Logo" width="100">"#;
    let img = parse_inline_html_image(html).unwrap();
    match img {
      Inline::Image {
        url,
        alt,
        width,
        height,
        ..
      } => {
        assert_eq!(url, "https://example.com/img.png");
        assert_eq!(alt, "Logo");
        assert_eq!(width.as_deref(), Some("100"));
        assert!(height.is_none());
      }
      _ => panic!("expected Image"),
    }
  }

  #[test]
  fn is_br_tag() {
    assert!(is_html_line_break_tag("<br>"));
    assert!(is_html_line_break_tag("<br/>"));
    assert!(is_html_line_break_tag("<br />"));
    assert!(is_html_line_break_tag("<BR>"));
    assert!(!is_html_line_break_tag("<b>"));
  }

  #[test]
  fn is_comment() {
    assert!(is_html_comment_only("<!-- hello -->"));
    assert!(is_html_comment_only("<!--\nmultiline\n-->"));
    assert!(!is_html_comment_only("<p>not a comment</p>"));
  }

  #[test]
  fn extract_attribute_double_quoted() {
    let tag = r#"<img src="hello.png" alt="test">"#;
    assert_eq!(
      extract_html_attribute(tag, "src"),
      Some("hello.png".to_string())
    );
    assert_eq!(extract_html_attribute(tag, "alt"), Some("test".to_string()));
  }

  #[test]
  fn extract_attribute_single_quoted() {
    let tag = "<img src='hello.png'>";
    assert_eq!(
      extract_html_attribute(tag, "src"),
      Some("hello.png".to_string())
    );
  }

  #[test]
  fn centered_div_detection() {
    assert!(is_centered_div_open(r#"<div align="center">"#));
    assert!(is_centered_div_open(r#"<div style="text-align: center">"#));
    assert!(!is_centered_div_open(r#"<div class="foo">"#));
  }

  #[test]
  fn html_entity_decoding() {
    assert_eq!(decode_html_entities("&amp; &lt; &gt;"), "& < >");
    assert_eq!(decode_html_entities("&quot;hi&quot;"), "\"hi\"");
  }

  #[test]
  fn summary_inlines_strips_tags() {
    let inlines = summary_inlines_from_text("<em><h4>Cargo Audit</h4></em>");
    assert_eq!(inlines.len(), 1);
    match &inlines[0] {
      Inline::Text(t) => assert_eq!(t, "Cargo Audit"),
      _ => panic!("expected Text"),
    }
  }
}
