# leaf Configuration & Theme Reference

## Config File Location

| Platform | Path |
|----------|------|
| Linux/macOS | `~/.config/leaf/config.toml` (or `$XDG_CONFIG_HOME/leaf/config.toml`) |
| Windows | `%APPDATA%\leaf\config.toml` |
| Termux | `~/.config/leaf/config.toml` |

Create/open with `leaf --config`.

## Config Options

```toml
# Color theme (built-in name or custom file path)
# Built-in: arctic, forest, ocean, solarized-dark
# Default: ocean
theme = "ocean"

# Default editor for Ctrl+E
# Any editor in PATH: nano, vim, nvim, micro, helix, emacs, code, subl, zed
# For paths with spaces: editor = 'C:\Program Files\Notepad++\notepad++.exe'
# editor = "nano"

# Maximum content width (columns, min 20)
# Default: terminal width (no limit)
# width = 80

# Auto-reload when opening a file (ignored for stdin)
# Default: false
watch = false

# Extra file types shown in file picker (without dot)
# Code files get syntax highlighting; text files render as Markdown
# Default: [] (Markdown only)
# extras = ["txt", "csv", "rs", "java", "json", "yaml"]
```

## Environment Variables

| Variable | Effect | Priority |
|----------|--------|----------|
| `LEAF_THEME` | Override config theme | Below `--theme`, above config |
| `LEAF_WIDTH` | Override config width (≥ 20) | Below `--width`, above config |
| `LEAF_EDITOR` | Override config editor | Below `--editor`, above config |

## CLI Flags

```
leaf [OPTIONS] [file.md | directory]

  -h, --help                   Show help
  -V, --version                Show version
  -w, --watch                  Watch file for changes
      --theme <NAME>           Theme preset or custom theme path
  -e, --editor <NAME>          External editor
      --inline [SPEC]          Render to stdout [ansi|plain][:<width>]
      --width <N>              Max content width (min 20)
      --picker                 Directory browser picker
      --config                 Open/create config in editor
      --update                 Self-update
      --auto-complete [SPEC]   Install/dump shell completions
      --debug-input            Debug logging to leaf-debug.log
```

## Custom Theme File Structure

A custom theme TOML inherits from a `base` preset and overrides specific colors. Only changed keys are needed.

### Top-Level Keys

```toml
base = "ocean"                    # REQUIRED: arctic | forest | ocean | solarized-dark
syntax = "base16-ocean.dark"      # syntect theme name for code blocks
```

### `[ui]` Section — All Available Keys

```toml
[ui]
content_bg = "#282828"            # main content background
toc_bg = "#1d2021"                # TOC sidebar background
toc_border = "#504945"            # TOC border color
scrollbar_hover = "#fabd2f"       # scrollbar hover highlight
status_bg = "#1d2021"             # status bar background
status_separator = "#928374"      # status bar separator
status_brand_fg = "#282828"       # "leaf" brand text
status_brand_bg = "#fabd2f"       # "leaf" brand background
status_filename_fg = "#ebdbb2"    # filename text
status_filename_bg = "#3c3836"    # filename background
status_watch_fg = "#b8bb26"       # watch indicator text
status_watch_bg = "#32361a"       # watch indicator background
status_reloaded_fg = "#282828"    # reload flash text
status_reloaded_bg = "#b8bb26"    # reload flash background
status_search_fg = "#fabd2f"      # search counter text
status_search_bg = "#3c3836"      # search counter background
status_success_fg = "#b8bb26"     # success message text
status_success_bg = "#32361a"     # success message background
status_warning_fg = "#fabd2f"     # warning message text
status_error_fg = "#fb4934"       # error message text
status_error_bg = "#3c1f1e"       # error message background
status_shortcut_fg = "#928374"    # keyboard shortcut hints
status_percent_fg = "#fabd2f"     # scroll percentage
toc_header_fg = "#928374"         # TOC header text
toc_active_bg = "#3c3836"         # TOC active item background
toc_inactive_bg = "#1d2021"       # TOC inactive item background
toc_accent = "#fe8019"            # TOC accent color
toc_index_inactive = "#665c54"    # TOC index number (inactive)
toc_primary_active = "#fbf1c7"    # TOC primary heading (active)
toc_primary_inactive = "#d5c4a1"  # TOC primary heading (inactive)
toc_secondary_inactive = "#665c54"           # TOC secondary heading (inactive)
toc_secondary_text_active = "#ebdbb2"        # TOC secondary text (active)
toc_secondary_text_inactive = "#928374"      # TOC secondary text (inactive)
```

### `[markdown]` Section — All Available Keys

```toml
[markdown]
text = "#ebdbb2"                  # body text
strong_text = "#fbf1c7"           # bold text
blockquote_text = "#d5c4a1"       # blockquote text
blockquote_marker = "#928374"     # blockquote "▏" marker

# Headings
heading_1 = "#fabd2f"
heading_2 = "#b8bb26"
heading_3 = "#83a598"
heading_4 = "#d3869b"
heading_other = "#fe8019"         # h5, h6
heading_underline = "#504945"     # heading underline

# Code
code_frame = "#665c54"            # code block border
code_label = "#928374"            # code block language label
code_gutter = "#665c54"           # line number gutter
inline_code_fg = "#fe8019"        # inline code text
inline_code_bg = "#3c3836"        # inline code background

# Lists
list_level_1 = "#b8bb26"          # bullet level 1
list_level_2 = "#83a598"          # bullet level 2
list_level_3 = "#d3869b"          # bullet level 3
ordered_list = "#fabd2f"          # ordered list numbers

# Tables
table_border = "#665c54"
table_separator = "#504945"
table_header = "#fabd2f"
table_cell = "#ebdbb2"

# Links
link_icon = "#83a598"             # link icon
link_text = "#83a598"             # link text
link_hover = "#b8d4c0"            # link hover state

# Search
search_highlight_bg = "#665c54"   # search highlight (all matches)
search_match_bg = "#fabd2f"       # search match (current match)

# Rules
rule = "#504945"                  # horizontal rule

# LaTeX
latex_inline_fg = "#d3869b"
latex_inline_bg = "#3c3836"
latex_block_fg = "#d3869b"

# Mermaid diagrams
mermaid_keyword = "#fe8019"
mermaid_arrow = "#83a598"
mermaid_label = "#b8bb26"
mermaid_block_fg = "#ebdbb2"

# Task lists
task_checked = "#b8bb26"
task_unchecked = "#928374"

# Mark (==highlight==)
mark_fg = "#ebdbb2"
mark_bg = "#79740e"

# GitHub-style alerts
alert_note = "#83a598"
alert_tip = "#b8bb26"
alert_important = "#d3869b"
alert_warning = "#fabd2f"
alert_caution = "#fb4934"
```

### Color Format

Colors use hex strings: `"#RRGGBB"`. Named ANSI colors are also supported:
`black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `gray`, `darkgray`,
`lightred`, `lightgreen`, `lightyellow`, `lightblue`, `lightmagenta`, `lightcyan`, `white`.

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `d` / PgDn | Page down (20 lines) |
| `u` / PgUp | Page up (20 lines) |
| `g` / Home | Top |
| `G` / End | Bottom |
| `t` | Toggle TOC sidebar |
| `Shift+T` | Open theme picker |
| `Shift+E` | Open editor picker |
| `Shift+P` | Open file browser |
| `Ctrl+E` | Open in editor |
| `Ctrl+P` | Open fuzzy picker |
| `Ctrl+F` / `/` | Find |
| `Ctrl+Click` | Open link |
| `Dbl-Click` | Copy link |
| `n` / `N` | Next / prev match |
| `?` | Show help popup |
| `r` | Force reload (watch mode) |
| `q` | Quit |

## Module Architecture

```
src/
├── main.rs              # entrypoint: CLI → config → input → parse → TUI/inline
├── cli.rs               # argument parsing
├── config.rs            # TOML config loading
├── inline.rs            # --inline stdout rendering (ANSI/plain)
├── terminal.rs          # raw mode / alternate screen lifecycle
├── editor.rs            # editor detection and launch
├── update.rs            # self-update with SHA256 verification
├── app/                 # central App state (document, TOC, search, watch, pickers)
├── markdown/            # pulldown-cmark → ratatui pipeline (core rendering)
├── render/              # ratatui TUI drawing (content, TOC, status, popups)
├── runtime/             # 50ms poll event loop, keyboard/mouse handling
└── theme/               # preset definitions, TOML theme resolution, color parsing
```

Execution flow: CLI parse → config load → theme resolve → document load → markdown parse → TUI event loop (or inline stdout exit).
