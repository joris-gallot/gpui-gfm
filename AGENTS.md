# Agent Guide

## Required Workflow

1. Use Context7 MCP for library/API docs, setup/config, and codegen guidance.
2. Keep changes minimal and scoped to the requested task.
3. For each feature/fix, add or update tests.
4. Validate before handoff:
   - `cargo check`
   - `cargo test`
5. Every GFM spec feature must have a matching example in the playground `SAMPLE_MARKDOWN` (`crates/playground/main.rs`).
6. Every render option implemented must have an example in the playground demonstrating its effect.

## Code Rules

- Prefer `rg` for search
