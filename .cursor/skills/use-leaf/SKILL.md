---
name: use-leaf
description: >-
  Guide for using leaf terminal Markdown previewer as a CLI tool and understanding
  its tech stack for building similar TUI Markdown renderers. Use when integrating
  leaf into scripts, fzf, Vim, CI pipelines, or when building a terminal Markdown
  rendering pipeline with pulldown-cmark + ratatui + syntect.
---

# Using leaf — Terminal Markdown Previewer

leaf is a **CLI binary** (not a Rust library). It has no `[lib]` section and all symbols are `pub(crate)`. Use it via shell invocation or `--inline` mode for programmatic integration.

## Quick Start

### Install

```bash
# macOS / Linux / Termux
curl -fsSL https://raw.githubusercontent.com/RivoLink/leaf/main/scripts/install.sh | sh

# npm (cross-platform)
npm install -g @rivolink/leaf

# Arch Linux
yay -S leaf-markdown-viewer

# From source
cargo build --release
```

### Basic Usage

```bash
leaf README.md                    # interactive TUI preview
leaf -w notes.md                  # watch mode (auto-reload on save)
leaf                              # fuzzy Markdown file picker
leaf --picker                     # directory browser picker
cat file.md | leaf                # stdin pipe
echo '# Hello' | leaf            # inline pipe
```

## Inline Mode (Scriptable API)

`--inline` is the primary programmatic interface — renders Markdown to stdout without TUI:

```bash
leaf --inline README.md                         # auto format (ANSI if tty, plain otherwise)
leaf --inline ansi README.md                    # force ANSI colors
leaf --inline plain README.md                   # force plain text
leaf --inline 60 README.md                      # set width to 60 columns
leaf --inline ansi:80 README.md                 # ANSI + specific width
cat README.md | leaf --inline                   # from stdin
```

Spec format: `[ansi|plain][:<width>]` — default auto-detects tty.

## Configuration

Config path: `~/.config/leaf/config.toml` (or `$XDG_CONFIG_HOME/leaf/config.toml`)

```toml
theme = "ocean"              # arctic | forest | ocean | solarized-dark | path/to/custom.toml
editor = "nano"              # any editor in PATH
watch = false                # auto-reload on open
width = 80                   # max content width (min 20)
extras = ["txt", "rs"]       # extra file types in picker
```

Priority: CLI flag > env var (`LEAF_THEME`, `LEAF_WIDTH`, `LEAF_EDITOR`) > config.toml > defaults

Run `leaf --config` to create/open config in your editor.

## Custom Themes

Create a TOML file inheriting from a built-in preset:

```toml
base = "ocean"
syntax = "base16-ocean.dark"

[ui]
content_bg = "#282828"
toc_accent = "#fe8019"

[markdown]
text = "#ebdbb2"
heading_1 = "#fabd2f"
inline_code_fg = "#b8bb26"
```

Set in config: `theme = "/path/to/custom-theme.toml"`. See [reference.md](reference.md) for all color keys.

## Tech Stack Pattern

leaf's rendering pipeline is a reusable architecture for terminal Markdown renderers:

```
pulldown-cmark (parse) → ratatui Line/Span (style) → crossterm (display)
                          ↳ syntect (code highlighting)
                          ↳ unicodeit (LaTeX → Unicode)
                          ↳ mmdflux (Mermaid → ASCII)
```

### Key Dependencies

| Crate | Version | Role |
|-------|---------|------|
| `pulldown-cmark` | 0.12 | CommonMark parser (event-based) |
| `ratatui` | 0.30 | TUI framework (`Line<'static>`, `Span<'static>`, `Style`) |
| `crossterm` | 0.29 | Terminal I/O, raw mode, events |
| `syntect` | 5.2 | Syntax highlighting (onig regex) |
| `unicodeit` | 0.2 | LaTeX → Unicode conversion |
| `mmdflux` | 2.5 | Mermaid diagram ASCII rendering |
| `anyhow` | 1.0 | Error handling |
| `serde` + `toml` | 1.0 / 0.8 | Config deserialization |

### Core Rendering Pattern (from `src/markdown/mod.rs`)

The central loop processes `pulldown-cmark` events into `ratatui` styled lines:

```rust
use pulldown_cmark::{Event, Options, Parser};
use ratatui::text::{Line, Span};

let parser = Parser::new_ext(&source, Options::all());
let mut lines: Vec<Line<'static>> = Vec::new();
let mut spans: Vec<Span<'static>> = Vec::new();

for event in parser {
    match event {
        Event::Start(Tag::Heading { level, .. }) => { /* start heading state */ }
        Event::End(TagEnd::Heading(_)) => { /* flush spans → styled heading line */ }
        Event::Text(text) => { /* push styled span */ }
        Event::Code(text) => { /* push inline code span with bg color */ }
        Event::Start(Tag::CodeBlock(kind)) => { /* start code buffer */ }
        Event::End(TagEnd::CodeBlock) => { /* syntect highlight → push lines */ }
        // ... tables, lists, blockquotes, LaTeX, Mermaid
        _ => {}
    }
}
```

### ANSI Serialization Pattern (from `src/inline.rs`)

Convert `ratatui` `Style` to raw ANSI escape codes for non-TUI output:

```rust
// Style → "\x1b[38;2;r;g;b m" (RGB foreground)
// Style → "\x1b[48;5;n m" (256-color background)
// Modifier::BOLD → "\x1b[1m", ITALIC → "\x1b[3m"
// Reset → "\x1b[0m"
```

Width-aware line wrapping with `unicode-width` for CJK/emoji support.

### Theme Inheritance Pattern (from `src/theme/`)

```
Built-in preset (Arctic|Forest|OceanDark|SolarizedDark)
    ↓ base = "ocean"
Custom TOML overrides only changed colors
    ↓ RwLock<ThemeSelection>
Global thread-safe theme state
```

## Fork for Library Use

leaf is a binary-only crate — no `lib.rs`, no public API. To use its rendering engine as a Rust library:

**Quick fork path (~3-5 dev-days):**

1. Add `[lib]` section to `Cargo.toml` + create `src/lib.rs`
2. Promote Tier 1 functions to `pub`: `parse_markdown`, `write_lines`, theme types
3. Fix 2 couplings: `TableBuf::render` global theme → explicit param; `FileState` move out of `app`
4. Replace tuple returns with `ParseOutput` struct

**Recommended: use `leaf-core` (stable facade crate):**

```rust
use leaf_core::{render_to_ansi, MarkdownRenderer};

// One-shot
let ansi = render_to_ansi("# Hello\n\nworld", 80);

// Reusable (caches syntax/theme assets)
let renderer = MarkdownRenderer::new();
let output = renderer.render("**bold** and `code`", 80);
let ansi = renderer.render_to_ansi("# Title", 80);
let plain = renderer.render_to_plain("# Title", 80);
```

```toml
# Cargo.toml
leaf-core = { path = "../leaf/crates/leaf-core" }
# or git:
leaf-core = { git = "https://github.com/StrayDragon/leaf" }
```

Key types: `MarkdownRenderer`, `ParseOutput`, `TocEntry`, `LinkSpan`, `ThemePreset`.

**Architecture:** `leaf-core` → `leaf` (lib) → `pulldown-cmark` + `ratatui` + `syntect`.
Consumers never depend on `leaf` directly; `leaf-core` absorbs upstream changes.

See [fork-guide.md](fork-guide.md) for detailed refactoring steps, type inventory, and effort estimates.

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Windows: missing VC++ runtime | Install from https://aka.ms/vc14/vc_redist.x64.exe |
| Windows: update fails (exe locked) | Close all leaf sessions, rerun PowerShell installer |
| `--inline` empty output | Ensure file exists or stdin has content |
| Watch mode not reloading | Check file is saved (not just buffer change); stdin can't be watched |
| Theme not applying | Check priority: `--theme` > `LEAF_THEME` env > config.toml |
| Wide chars break layout | Update to latest version; `unicode-width` 0.2 handles CJK |

## Additional Resources

- For complete configuration reference and all theme color keys: [reference.md](reference.md)
- For integration examples (fzf, Vim, CI, scripts, AI workflow): [examples.md](examples.md)
- For forking leaf to expose a Rust library API: [fork-guide.md](fork-guide.md)
