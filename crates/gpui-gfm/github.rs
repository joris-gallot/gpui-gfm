//! GitHub-specific features: blob line references, issue reference auto-linking.

use std::sync::Arc;

use crate::types::Inline;

/// A parsed reference to a specific line range in a GitHub blob URL.
///
/// Example: `https://github.com/owner/repo/blob/main/src/lib.rs#L10-L20`
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GithubBlobLineReference {
  pub url: String,
  pub owner: String,
  pub repo: String,
  pub reference: String,
  pub path: String,
  pub start_line: usize,
  pub end_line: usize,
}

/// Context for auto-linking `#123` issue references.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GithubIssueReferenceContext {
  pub owner: Arc<str>,
  pub repo: Arc<str>,
}

/// Parse a GitHub blob URL into its components with line reference.
///
/// Handles URLs like:
/// - `https://github.com/owner/repo/blob/ref/path/to/file#L10`
/// - `https://github.com/owner/repo/blob/ref/path/to/file#L10-L20`
pub fn parse_github_blob_line_reference(url: &str) -> Option<GithubBlobLineReference> {
  let url_str = url.trim();

  // Strip the scheme
  let rest = url_str
    .strip_prefix("https://github.com/")
    .or_else(|| url_str.strip_prefix("http://github.com/"))?;

  // Split at the fragment (#L...)
  let (path_part, fragment) = rest.split_once('#')?;

  // Parse line range from fragment
  let (start_line, end_line) = parse_line_fragment(fragment)?;

  // Split path: owner/repo/blob/ref/path...
  let mut parts = path_part.splitn(5, '/');
  let owner = parts.next()?.to_string();
  let repo = parts.next()?.to_string();
  let blob_or_tree = parts.next()?;
  if blob_or_tree != "blob" {
    return None;
  }
  let reference = parts.next()?.to_string();
  let file_path = parts.next()?.to_string();

  if owner.is_empty() || repo.is_empty() || reference.is_empty() || file_path.is_empty() {
    return None;
  }

  Some(GithubBlobLineReference {
    url: url_str.to_string(),
    owner,
    repo,
    reference,
    path: file_path,
    start_line,
    end_line,
  })
}

/// Parse a `L10` or `L10-L20` fragment into (start, end) line numbers.
fn parse_line_fragment(fragment: &str) -> Option<(usize, usize)> {
  let fragment = fragment.trim();

  if let Some((start_s, end_s)) = fragment.split_once('-') {
    let start = start_s.strip_prefix('L').or(start_s.strip_prefix('l'))?;
    let end = end_s.strip_prefix('L').or(end_s.strip_prefix('l'))?;
    let start: usize = start.parse().ok()?;
    let end: usize = end.parse().ok()?;
    if start == 0 || end == 0 || end < start {
      return None;
    }
    Some((start, end))
  } else {
    let line = fragment.strip_prefix('L').or(fragment.strip_prefix('l'))?;
    let line: usize = line.parse().ok()?;
    if line == 0 {
      return None;
    }
    Some((line, line))
  }
}

/// Expand `#123` issue references in a list of inlines.
///
/// Only `Inline::Text` nodes are scanned. References inside `Code`, `Link`,
/// `Strong`, `Emphasis`, or `Strikethrough` are recursively processed (except
/// `Code` and `Link` which are left untouched).
///
/// A valid issue reference is `#` followed by 1+ digits, where the `#` is
/// either at the start of the text or preceded by a whitespace/punctuation
/// character (not `&` which would be an HTML entity like `&#123;`).
pub fn expand_issue_references(
  inlines: &[Inline],
  ctx: &GithubIssueReferenceContext,
) -> Vec<Inline> {
  let mut result = Vec::with_capacity(inlines.len());

  for inline in inlines {
    match inline {
      Inline::Text(text) => {
        split_issue_refs(text, ctx, &mut result);
      }
      // Recurse into formatting nodes.
      Inline::Strong(children) => {
        result.push(Inline::Strong(expand_issue_references(children, ctx)));
      }
      Inline::Emphasis(children) => {
        result.push(Inline::Emphasis(expand_issue_references(children, ctx)));
      }
      Inline::Strikethrough(children) => {
        result.push(Inline::Strikethrough(expand_issue_references(
          children, ctx,
        )));
      }
      // Don't touch code spans or existing links — clone as-is.
      other => {
        result.push(other.clone());
      }
    }
  }

  result
}

/// Split a text string at `#\d+` boundaries, emitting `Text` and `Link` inlines.
fn split_issue_refs(text: &str, ctx: &GithubIssueReferenceContext, out: &mut Vec<Inline>) {
  let bytes = text.as_bytes();
  let mut last_end = 0;

  let mut i = 0;
  while i < bytes.len() {
    if bytes[i] == b'#' {
      // Guard: `#` must be at start or preceded by whitespace/punctuation (not `&`).
      if i > 0 {
        let prev = bytes[i - 1];
        // Allow whitespace and common punctuation, but not `&` (HTML entities like &#123;)
        // and not alphanumeric (would mean it's part of a word like foo#123).
        if prev.is_ascii_alphanumeric() || prev == b'&' {
          i += 1;
          continue;
        }
      }

      // Scan digits after `#`.
      let num_start = i + 1;
      let mut num_end = num_start;
      while num_end < bytes.len() && bytes[num_end].is_ascii_digit() {
        num_end += 1;
      }

      // Must have at least one digit.
      if num_end > num_start {
        // Guard: the character after the digits must not be alphanumeric
        // (e.g. `#123abc` is not a valid reference).
        if num_end < bytes.len() && bytes[num_end].is_ascii_alphanumeric() {
          i += 1;
          continue;
        }

        let number = &text[num_start..num_end];

        // Emit any text before this reference.
        if i > last_end {
          out.push(Inline::Text(text[last_end..i].to_string()));
        }

        // Emit the link.
        let display = format!("#{number}");
        let url = format!(
          "https://github.com/{}/{}/issues/{number}",
          ctx.owner, ctx.repo
        );
        out.push(Inline::Link {
          url,
          title: None,
          content: vec![Inline::Text(display)],
        });

        last_end = num_end;
        i = num_end;
        continue;
      }
    }

    i += 1;
  }

  // Emit trailing text.
  if last_end < text.len() {
    out.push(Inline::Text(text[last_end..].to_string()));
  }
}

/// Extract all GitHub blob line references from a markdown text.
pub fn extract_github_blob_line_references(text: &str) -> Vec<GithubBlobLineReference> {
  let mut refs = Vec::new();
  for word in text.split_whitespace() {
    // Strip common markdown link punctuation
    let cleaned = word.trim_matches(|c: char| c == '(' || c == ')' || c == '<' || c == '>');
    if let Some(r) = parse_github_blob_line_reference(cleaned) {
      refs.push(r);
    }
  }
  refs
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_single_line_reference() {
    let url = "https://github.com/owner/repo/blob/main/src/lib.rs#L42";
    let r = parse_github_blob_line_reference(url).unwrap();
    assert_eq!(r.owner, "owner");
    assert_eq!(r.repo, "repo");
    assert_eq!(r.reference, "main");
    assert_eq!(r.path, "src/lib.rs");
    assert_eq!(r.start_line, 42);
    assert_eq!(r.end_line, 42);
  }

  #[test]
  fn parse_line_range_reference() {
    let url = "https://github.com/owner/repo/blob/abc123/path/file.rs#L10-L20";
    let r = parse_github_blob_line_reference(url).unwrap();
    assert_eq!(r.start_line, 10);
    assert_eq!(r.end_line, 20);
  }

  #[test]
  fn rejects_non_blob_url() {
    let url = "https://github.com/owner/repo/tree/main/src#L5";
    assert!(parse_github_blob_line_reference(url).is_none());
  }

  #[test]
  fn rejects_no_fragment() {
    let url = "https://github.com/owner/repo/blob/main/src/lib.rs";
    assert!(parse_github_blob_line_reference(url).is_none());
  }

  #[test]
  fn extract_multiple_references() {
    let text =
      "See https://github.com/a/b/blob/main/x.rs#L1-L5 and https://github.com/c/d/blob/v1/y.rs#L10";
    let refs = extract_github_blob_line_references(text);
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].path, "x.rs");
    assert_eq!(refs[1].path, "y.rs");
  }

  // --- Issue reference expansion tests ---

  fn ctx() -> GithubIssueReferenceContext {
    GithubIssueReferenceContext {
      owner: "zed-industries".into(),
      repo: "zed".into(),
    }
  }

  #[test]
  fn issue_ref_simple() {
    let inlines = vec![Inline::Text("See #123 for details".into())];
    let result = expand_issue_references(&inlines, &ctx());
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], Inline::Text("See ".into()));
    assert_eq!(
      result[1],
      Inline::Link {
        url: "https://github.com/zed-industries/zed/issues/123".into(),
        title: None,
        content: vec![Inline::Text("#123".into())],
      }
    );
    assert_eq!(result[2], Inline::Text(" for details".into()));
  }

  #[test]
  fn issue_ref_at_start() {
    let inlines = vec![Inline::Text("#42 is fixed".into())];
    let result = expand_issue_references(&inlines, &ctx());
    assert_eq!(result.len(), 2);
    assert!(matches!(&result[0], Inline::Link { url, .. } if url.ends_with("/42")));
    assert_eq!(result[1], Inline::Text(" is fixed".into()));
  }

  #[test]
  fn issue_ref_at_end() {
    let inlines = vec![Inline::Text("Fixed in #99".into())];
    let result = expand_issue_references(&inlines, &ctx());
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], Inline::Text("Fixed in ".into()));
    assert!(matches!(&result[1], Inline::Link { url, .. } if url.ends_with("/99")));
  }

  #[test]
  fn issue_ref_not_alpha() {
    // #abc is not a valid issue reference
    let inlines = vec![Inline::Text("See #abc here".into())];
    let result = expand_issue_references(&inlines, &ctx());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], Inline::Text("See #abc here".into()));
  }

  #[test]
  fn issue_ref_not_in_word() {
    // foo#123 should NOT be expanded (preceded by alphanumeric)
    let inlines = vec![Inline::Text("foo#123 bar".into())];
    let result = expand_issue_references(&inlines, &ctx());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], Inline::Text("foo#123 bar".into()));
  }

  #[test]
  fn issue_ref_not_html_entity() {
    // &#123; should NOT be expanded (preceded by &)
    let inlines = vec![Inline::Text("char &#123; end".into())];
    let result = expand_issue_references(&inlines, &ctx());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], Inline::Text("char &#123; end".into()));
  }

  #[test]
  fn issue_ref_not_followed_by_alpha() {
    // #123abc should NOT be expanded
    let inlines = vec![Inline::Text("see #123abc".into())];
    let result = expand_issue_references(&inlines, &ctx());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], Inline::Text("see #123abc".into()));
  }

  #[test]
  fn issue_ref_in_code_untouched() {
    let inlines = vec![Inline::Code("#123".into())];
    let result = expand_issue_references(&inlines, &ctx());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], Inline::Code("#123".into()));
  }

  #[test]
  fn issue_ref_in_link_untouched() {
    let inlines = vec![Inline::Link {
      url: "https://example.com".into(),
      title: None,
      content: vec![Inline::Text("#123".into())],
    }];
    let result = expand_issue_references(&inlines, &ctx());
    assert_eq!(result.len(), 1);
    // The link itself is unchanged — its content is NOT scanned.
    assert!(matches!(&result[0], Inline::Link { .. }));
  }

  #[test]
  fn issue_ref_in_bold() {
    let inlines = vec![Inline::Strong(vec![Inline::Text("fix #42".into())])];
    let result = expand_issue_references(&inlines, &ctx());
    assert_eq!(result.len(), 1);
    match &result[0] {
      Inline::Strong(children) => {
        assert_eq!(children.len(), 2);
        assert_eq!(children[0], Inline::Text("fix ".into()));
        assert!(matches!(&children[1], Inline::Link { url, .. } if url.ends_with("/42")));
      }
      _ => panic!("expected Strong"),
    }
  }

  #[test]
  fn issue_ref_multiple() {
    let inlines = vec![Inline::Text("#1 and #2".into())];
    let result = expand_issue_references(&inlines, &ctx());
    // Link + " and " + Link
    assert_eq!(result.len(), 3);
    assert!(matches!(&result[0], Inline::Link { url, .. } if url.ends_with("/1")));
    assert_eq!(result[1], Inline::Text(" and ".into()));
    assert!(matches!(&result[2], Inline::Link { url, .. } if url.ends_with("/2")));
  }

  #[test]
  fn issue_ref_after_paren() {
    // (#123) — `#` preceded by `(` which is punctuation → should expand
    let inlines = vec![Inline::Text("(#123)".into())];
    let result = expand_issue_references(&inlines, &ctx());
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], Inline::Text("(".into()));
    assert!(matches!(&result[1], Inline::Link { url, .. } if url.ends_with("/123")));
    assert_eq!(result[2], Inline::Text(")".into()));
  }
}
