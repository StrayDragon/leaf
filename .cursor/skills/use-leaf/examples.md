# leaf Integration Examples

## fzf Preview

### Markdown file picker with live preview

```bash
find . -name '*.md' | fzf --preview 'leaf --inline ansi:$FZF_PREVIEW_COLUMNS {}'
```

### Multi-type file picker

```bash
find . \( -name '*.md' -o -name '*.rs' -o -name '*.py' \) | \
  fzf --preview 'leaf --inline ansi:$FZF_PREVIEW_COLUMNS {}'
```

### Git-tracked Markdown picker

```bash
git ls-files '*.md' | fzf --preview 'leaf --inline ansi:$FZF_PREVIEW_COLUMNS {}'
```

## Vim / Neovim

### Live preview in vertical split

```vim
" ~/.vimrc or ~/.config/nvim/init.vim
nnoremap <Leader>md :vertical botright terminal leaf -w %<CR>
```

Use `\md` to open preview. `Ctrl+w,h` to switch back to editor.

### Neovim floating window preview

```lua
vim.keymap.set('n', '<Leader>md', function()
  local file = vim.fn.expand('%:p')
  vim.cmd('vsplit | terminal leaf -w ' .. vim.fn.shellescape(file))
end)
```

## AI Workflow

### Stream AI output with live preview

```bash
# Terminal 1: generate
aichat "explain Rust lifetimes" > notes.md

# Terminal 2: live watch
leaf --watch notes.md
```

### One-shot AI preview

```bash
claude "summarize this project" | leaf
```

### AI + fzf: browse generated docs

```bash
for topic in "closures" "traits" "lifetimes"; do
  claude "explain Rust $topic" > "docs/$topic.md"
done
find docs -name '*.md' | fzf --preview 'leaf --inline ansi:$FZF_PREVIEW_COLUMNS {}'
```

## Shell Scripts

### Batch render Markdown to plain text

```bash
#!/bin/bash
for file in docs/*.md; do
  leaf --inline plain "$file" > "${file%.md}.txt"
done
```

### Render and pipe to pager

```bash
leaf --inline ansi README.md | less -R
```

### Markdown preview in tmux popup

```bash
tmux display-popup -w 80% -h 80% "leaf README.md"
```

### Generate and preview with watch

```bash
#!/bin/bash
output="report.md"
echo "# Report" > "$output"
leaf -w "$output" &
LEAF_PID=$!

# ... append content to $output ...
echo "## Results" >> "$output"
echo "Data: OK" >> "$output"

# leaf auto-reloads on each save
wait $LEAF_PID
```

## CI / Automation

### GitHub Actions: render changelog

```yaml
- name: Render changelog
  run: |
    npm install -g @rivolink/leaf
    leaf --inline plain CHANGELOG.md > changelog.txt
```

### Pre-commit hook: preview before commit

```bash
#!/bin/bash
# .git/hooks/pre-commit
changed_md=$(git diff --cached --name-only --diff-filter=ACM '*.md')
if [ -n "$changed_md" ]; then
  echo "Changed Markdown files:"
  for f in $changed_md; do
    echo "--- $f ---"
    leaf --inline plain "$f" | head -20
  done
fi
```

## Custom Theme Workflow

### Create a Gruvbox-based theme

```bash
cat > ~/.config/leaf/gruvbox.toml << 'EOF'
base = "ocean"
syntax = "base16-ocean.dark"

[ui]
content_bg = "#282828"
toc_accent = "#fe8019"

[markdown]
text = "#ebdbb2"
heading_1 = "#fabd2f"
heading_2 = "#b8bb26"
inline_code_fg = "#fe8019"
inline_code_bg = "#3c3836"
EOF

# Set as default
sed -i 's/^theme = .*/theme = "gruvbox.toml"/' ~/.config/leaf/config.toml
```

### Test different themes

```bash
for theme in arctic forest ocean solarized-dark; do
  echo "=== $theme ==="
  leaf --theme "$theme" --inline ansi:60 README.md | head -30
  echo
done
```

## Building Similar Tools (Tech Stack Reference)

### Minimal Markdown → Terminal renderer

```rust
// Cargo.toml dependencies:
// pulldown-cmark = "0.12"
// ratatui = "0.30"
// crossterm = "0.29"
// syntect = { version = "5.2", features = ["default-syntaxes", "default-themes"] }

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

fn render_markdown(source: &str) -> Vec<Line<'static>> {
    let parser = Parser::new_ext(source, Options::all());
    let mut lines = Vec::new();
    let mut spans = Vec::new();
    let mut in_heading = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => { in_heading = true; }
            Event::End(TagEnd::Heading(_)) => {
                lines.push(Line::from(std::mem::take(&mut spans)));
                lines.push(Line::from(""));
                in_heading = false;
            }
            Event::Text(text) => {
                let style = if in_heading {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                spans.push(Span::styled(text.to_string(), style));
            }
            Event::End(TagEnd::Paragraph) => {
                lines.push(Line::from(std::mem::take(&mut spans)));
                lines.push(Line::from(""));
            }
            Event::Code(code) => {
                spans.push(Span::styled(
                    format!(" {} ", code),
                    Style::default().fg(Color::Cyan).bg(Color::DarkGray),
                ));
            }
            _ => {}
        }
    }
    lines
}
```

### Adding syntect code highlighting

```rust
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::as_24bit_terminal_escaped;

fn highlight_code(code: &str, lang: &str) -> Vec<Line<'static>> {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ss.find_syntax_by_token(lang)
        .unwrap_or_else(|| ss.find_syntax_plain_text());
    let theme = &ts.themes["base16-ocean.dark"];
    let mut h = HighlightLines::new(syntax, theme);

    code.lines().map(|line| {
        let ranges = h.highlight_line(line, &ss).unwrap();
        let spans: Vec<Span<'static>> = ranges.into_iter().map(|(style, text)| {
            let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
            Span::styled(text.to_string(), Style::default().fg(fg))
        }).collect();
        Line::from(spans)
    }).collect()
}
```
