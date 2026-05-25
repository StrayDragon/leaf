//! Streaming Markdown renderer for LLM token-by-token output.
//!
//! Three rendering tiers, each building on the previous:
//!
//! - **Debounced full reparse** — accumulate tokens, reparse at tick intervals
//! - **Incremental diff** — detect stable prefix, only expose changed tail
//! - **Dual-phase** — committed paragraphs get full Markdown rendering,
//!   the in-progress line gets lightweight ANSI styling
//!
//! # Usage
//!
//! ```rust
//! use leaf_core::streaming::StreamingRenderer;
//! use leaf_core::MarkdownRenderer;
//!
//! let renderer = MarkdownRenderer::new();
//! let mut stream = StreamingRenderer::new(renderer, 80);
//!
//! // LLM tokens arrive
//! stream.push("# Hello");
//! stream.push("\n\nSome **bold** text");
//!
//! // Periodic tick (call from event loop, e.g. every 100-200ms)
//! if let Some(update) = stream.tick() {
//!     // update.lines — full rendered lines
//!     // update.changed_from — first line index that changed
//!     // update.phase — which rendering tier produced this
//! }
//!
//! // Signal end of stream
//! let final_output = stream.finish();
//! ```

use crate::{Line, MarkdownRenderer, Span};
use ratatui::style::{Color, Modifier, Style};
use std::time::{Duration, Instant};

/// Minimum interval between full reparses during active streaming.
const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(150);

/// Describes which rendering phase produced the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RenderPhase {
    /// Full Markdown reparse of entire buffer (methods A/B).
    FullReparse,
    /// Dual-phase: committed prefix is precise Markdown, tail is approximate (method C).
    DualPhase,
    /// Final render after stream ends — always full precision.
    Final,
}

/// Incremental update returned by [`StreamingRenderer::tick`].
pub struct StreamUpdate<'a> {
    /// All rendered lines (full document).
    pub lines: &'a [Line<'static>],
    /// Index of the first line that differs from the previous tick's output.
    /// Lines before this index are unchanged (stable prefix).
    /// `0` means everything may have changed.
    pub changed_from: usize,
    /// Which rendering tier produced this update.
    pub phase: RenderPhase,
    /// Whether the stream has ended (no more tokens expected).
    pub finished: bool,
}

/// Streaming Markdown renderer for progressive LLM output.
///
/// Accumulates tokens and renders at controlled intervals,
/// with incremental diff detection and optional dual-phase rendering.
pub struct StreamingRenderer {
    renderer: MarkdownRenderer,
    width: usize,

    buffer: String,
    dirty: bool,
    finished: bool,

    last_lines: Vec<Line<'static>>,
    last_tick: Option<Instant>,
    debounce: Duration,

    dual_phase: bool,
    committed_len: usize,
    committed_lines: Vec<Line<'static>>,
}

impl StreamingRenderer {
    /// Create a streaming renderer with default debounce (150ms).
    pub fn new(renderer: MarkdownRenderer, width: usize) -> Self {
        Self {
            renderer,
            width,
            buffer: String::new(),
            dirty: false,
            finished: false,
            last_lines: Vec::new(),
            last_tick: None,
            debounce: DEFAULT_DEBOUNCE,
            dual_phase: false,
            committed_len: 0,
            committed_lines: Vec::new(),
        }
    }

    /// Enable dual-phase rendering (method C).
    ///
    /// When enabled, completed paragraphs (terminated by `\n\n`) are rendered
    /// with full Markdown precision and cached. The in-progress tail gets
    /// lightweight approximate styling.
    pub fn with_dual_phase(mut self, enabled: bool) -> Self {
        self.dual_phase = enabled;
        self
    }

    /// Set the debounce interval between reparses.
    pub fn with_debounce(mut self, debounce: Duration) -> Self {
        self.debounce = debounce;
        self
    }

    /// Set render width.
    pub fn set_width(&mut self, width: usize) {
        if self.width != width {
            self.width = width;
            self.dirty = true;
            self.committed_len = 0;
            self.committed_lines.clear();
        }
    }

    /// Append a token (or chunk of tokens) to the buffer.
    pub fn push(&mut self, token: &str) {
        if token.is_empty() || self.finished {
            return;
        }
        self.buffer.push_str(token);
        self.dirty = true;
    }

    /// Current accumulated source text.
    pub fn source(&self) -> &str {
        &self.buffer
    }

    /// Whether there are un-rendered changes since the last tick.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Whether the stream has been finished.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Total bytes accumulated so far.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Process a tick. Returns an update if rendering occurred.
    ///
    /// Call this from your event loop at a regular interval (e.g. every frame).
    /// Rendering only happens when:
    /// 1. The buffer is dirty (new tokens since last render), AND
    /// 2. At least `debounce` time has passed since the last render.
    pub fn tick(&mut self) -> Option<StreamUpdate<'_>> {
        if !self.dirty {
            return None;
        }

        let now = Instant::now();
        if let Some(last) = self.last_tick {
            if now.duration_since(last) < self.debounce {
                return None;
            }
        }

        self.last_tick = Some(now);
        self.dirty = false;

        let phase = if self.dual_phase {
            self.render_dual_phase()
        } else {
            self.render_full()
        };

        let changed_from = find_stable_prefix_len(&self.last_lines, &self.last_lines);

        Some(StreamUpdate {
            lines: &self.last_lines,
            changed_from,
            phase,
            finished: false,
        })
    }

    /// Force an immediate render, ignoring debounce.
    ///
    /// Useful when you need the latest output right now (e.g. user scrolled).
    pub fn force_tick(&mut self) -> StreamUpdate<'_> {
        self.dirty = false;
        self.last_tick = Some(Instant::now());

        let phase = if self.dual_phase {
            self.render_dual_phase()
        } else {
            self.render_full()
        };

        let changed_from = 0;

        StreamUpdate {
            lines: &self.last_lines,
            changed_from,
            phase,
            finished: false,
        }
    }

    /// Signal that the stream has ended. Returns the final fully-rendered output.
    ///
    /// After this, `push` and `tick` are no-ops.
    pub fn finish(&mut self) -> StreamUpdate<'_> {
        self.finished = true;
        self.committed_len = 0;
        self.committed_lines.clear();

        let output = self.renderer.render(&self.buffer, self.width);
        let old_lines = std::mem::replace(&mut self.last_lines, output.lines);
        let changed_from = find_stable_prefix_len(&old_lines, &self.last_lines);

        StreamUpdate {
            lines: &self.last_lines,
            changed_from,
            phase: RenderPhase::Final,
            finished: true,
        }
    }

    /// Reset the renderer for a new stream, keeping cached assets.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.dirty = false;
        self.finished = false;
        self.last_lines.clear();
        self.last_tick = None;
        self.committed_len = 0;
        self.committed_lines.clear();
    }

    // ── Internal rendering methods ───────────────────

    fn render_full(&mut self) -> RenderPhase {
        let output = self.renderer.render(&self.buffer, self.width);
        let old_lines = std::mem::replace(&mut self.last_lines, output.lines);
        let _ = old_lines;
        RenderPhase::FullReparse
    }

    fn render_dual_phase(&mut self) -> RenderPhase {
        let commit_boundary = find_last_paragraph_break(&self.buffer);

        if commit_boundary > self.committed_len {
            let committed_source = &self.buffer[..commit_boundary];
            let output = self.renderer.render(committed_source, self.width);
            self.committed_lines = output.lines;
            self.committed_len = commit_boundary;
        }

        let tail = &self.buffer[self.committed_len..];
        let tail_lines = render_approximate(tail);

        let mut combined = self.committed_lines.clone();
        combined.extend(tail_lines);
        self.last_lines = combined;

        RenderPhase::DualPhase
    }
}

// ── Incremental diff (method B) ──────────────────

/// Find the first line index where `old` and `new` diverge.
fn find_stable_prefix_len(old: &[Line<'_>], new: &[Line<'_>]) -> usize {
    old.iter()
        .zip(new.iter())
        .take_while(|(a, b)| lines_equal(a, b))
        .count()
}

fn lines_equal(a: &Line<'_>, b: &Line<'_>) -> bool {
    if a.spans.len() != b.spans.len() {
        return false;
    }
    a.spans
        .iter()
        .zip(b.spans.iter())
        .all(|(sa, sb)| sa.content == sb.content && sa.style == sb.style)
}

// ── Dual-phase approximate renderer (method C) ───

/// Find the byte offset of the last paragraph break (`\n\n`) in the source.
/// Returns 0 if no paragraph break found.
fn find_last_paragraph_break(source: &str) -> usize {
    if let Some(pos) = source.rfind("\n\n") {
        pos + 2
    } else {
        0
    }
}

/// Lightweight approximate rendering for in-progress text.
///
/// Applies basic inline styling without full Markdown parsing:
/// - `**bold**` → bold
/// - `` `code` `` → cyan on dark background
/// - `# heading` at line start → bold + color
/// - Everything else → default style
fn render_approximate(text: &str) -> Vec<Line<'static>> {
    text.lines()
        .map(|line| {
            if line.is_empty() {
                return Line::from("");
            }

            if let Some(stripped) = line.strip_prefix("# ") {
                return Line::from(Span::styled(
                    stripped.to_string(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            if let Some(stripped) = line.strip_prefix("## ") {
                return Line::from(Span::styled(
                    stripped.to_string(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            if let Some(stripped) = line.strip_prefix("### ") {
                return Line::from(Span::styled(
                    stripped.to_string(),
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            if line.starts_with("```") {
                return Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            let spans = parse_inline_approximate(line);
            Line::from(spans)
        })
        .collect()
}

fn parse_inline_approximate(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut chars = text.char_indices().peekable();
    let mut current = String::new();
    let mut style = Style::default();

    while let Some((i, ch)) = chars.next() {
        match ch {
            '`' => {
                if !current.is_empty() {
                    spans.push(Span::styled(std::mem::take(&mut current), style));
                    style = Style::default();
                }
                let code_start = i + 1;
                let mut code_end = code_start;
                while let Some(&(j, c)) = chars.peek() {
                    chars.next();
                    if c == '`' {
                        code_end = j;
                        break;
                    }
                    code_end = j + c.len_utf8();
                }
                let code_text = &text[code_start..code_end];
                spans.push(Span::styled(
                    format!(" {} ", code_text),
                    Style::default().fg(Color::Cyan).bg(Color::Black),
                ));
            }
            '*' if chars.peek().is_some_and(|&(_, c)| c == '*') => {
                chars.next();
                if !current.is_empty() {
                    spans.push(Span::styled(std::mem::take(&mut current), style));
                }
                let bold_start = chars
                    .peek()
                    .map(|&(j, _)| j)
                    .unwrap_or(text.len());
                let mut bold_end = bold_start;
                let mut found_close = false;
                while let Some(&(j, c)) = chars.peek() {
                    if c == '*' {
                        chars.next();
                        if chars.peek().is_some_and(|&(_, c2)| c2 == '*') {
                            chars.next();
                            bold_end = j;
                            found_close = true;
                            break;
                        }
                        bold_end = j + 1;
                    } else {
                        chars.next();
                        bold_end = j + c.len_utf8();
                    }
                }
                let bold_text = &text[bold_start..bold_end];
                if found_close {
                    spans.push(Span::styled(
                        bold_text.to_string(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                } else {
                    current.push_str("**");
                    current.push_str(bold_text);
                }
                style = Style::default();
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        spans.push(Span::styled(current, style));
    }

    if spans.is_empty() {
        spans.push(Span::raw(String::new()));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_tick() {
        let renderer = MarkdownRenderer::new();
        let mut stream = StreamingRenderer::new(renderer, 80)
            .with_debounce(Duration::ZERO);

        stream.push("# Hello\n\nworld");
        let update = stream.tick().expect("should render");

        assert!(!update.lines.is_empty());
        assert_eq!(update.phase, RenderPhase::FullReparse);
        assert!(!update.finished);
    }

    #[test]
    fn test_debounce_skips_rapid_ticks() {
        let renderer = MarkdownRenderer::new();
        let mut stream = StreamingRenderer::new(renderer, 80)
            .with_debounce(Duration::from_secs(10));

        stream.push("# Hello");
        let first = stream.tick();
        assert!(first.is_some());

        stream.push(" world");
        let second = stream.tick();
        assert!(second.is_none(), "should be debounced");
    }

    #[test]
    fn test_force_tick_ignores_debounce() {
        let renderer = MarkdownRenderer::new();
        let mut stream = StreamingRenderer::new(renderer, 80)
            .with_debounce(Duration::from_secs(10));

        stream.push("# Hello");
        stream.tick();
        stream.push(" world");

        let update = stream.force_tick();
        assert!(!update.lines.is_empty());
    }

    #[test]
    fn test_finish_produces_final() {
        let renderer = MarkdownRenderer::new();
        let mut stream = StreamingRenderer::new(renderer, 80)
            .with_debounce(Duration::ZERO);

        stream.push("# Title\n\nParagraph");
        let update = stream.finish();

        assert_eq!(update.phase, RenderPhase::Final);
        assert!(update.finished);
        assert!(!update.lines.is_empty());
    }

    #[test]
    fn test_dual_phase_rendering() {
        let renderer = MarkdownRenderer::new();
        let mut stream = StreamingRenderer::new(renderer, 80)
            .with_dual_phase(true)
            .with_debounce(Duration::ZERO);

        stream.push("# Complete paragraph\n\nThis is done.\n\nPartial tex");
        let update = stream.tick().expect("should render");

        assert_eq!(update.phase, RenderPhase::DualPhase);
        assert!(!update.lines.is_empty());
    }

    #[test]
    fn test_reset_clears_state() {
        let renderer = MarkdownRenderer::new();
        let mut stream = StreamingRenderer::new(renderer, 80)
            .with_debounce(Duration::ZERO);

        stream.push("# Hello");
        stream.tick();
        stream.reset();

        assert!(stream.is_empty());
        assert!(!stream.is_dirty());
        assert!(!stream.is_finished());
    }

    #[test]
    fn test_no_push_after_finish() {
        let renderer = MarkdownRenderer::new();
        let mut stream = StreamingRenderer::new(renderer, 80)
            .with_debounce(Duration::ZERO);

        stream.push("hello");
        stream.finish();
        let len_before = stream.len();

        stream.push("more");
        assert_eq!(stream.len(), len_before, "push after finish is no-op");
    }

    #[test]
    fn test_stable_prefix_detection() {
        let line_a = vec![
            Line::from("hello"),
            Line::from("world"),
            Line::from("changed"),
        ];
        let line_b = vec![
            Line::from("hello"),
            Line::from("world"),
            Line::from("different"),
        ];
        assert_eq!(find_stable_prefix_len(&line_a, &line_b), 2);
    }

    #[test]
    fn test_approximate_rendering() {
        let lines = render_approximate("# Heading\n\n**bold** and `code`");
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_paragraph_break_detection() {
        assert_eq!(find_last_paragraph_break("a\n\nb\n\nc"), 6);
        assert_eq!(find_last_paragraph_break("no break"), 0);
        assert_eq!(find_last_paragraph_break("a\n\n"), 3);
    }
}
