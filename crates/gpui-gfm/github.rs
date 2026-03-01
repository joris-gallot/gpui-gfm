//! GitHub-specific features: blob line references, issue reference auto-linking.

use std::sync::Arc;

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
    let text = "See https://github.com/a/b/blob/main/x.rs#L1-L5 and https://github.com/c/d/blob/v1/y.rs#L10";
    let refs = extract_github_blob_line_references(text);
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].path, "x.rs");
    assert_eq!(refs[1].path, "y.rs");
  }
}
