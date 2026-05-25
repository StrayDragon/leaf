//! # leaf-core — Stable facade for leaf's Markdown rendering pipeline
//!
//! This crate provides a minimal, stable API surface over the [`leaf`] library,
//! shielding consumers from internal changes when syncing with upstream.
//!
//! ## Quick start
//!
//! ```rust
//! use leaf_core::{render_to_ansi, MarkdownRenderer};
//!
//! // One-shot rendering
//! let ansi = render_to_ansi("# Hello\n\nworld", 80);
//! assert!(!ansi.is_empty());
//!
//! // Reusable renderer (avoids reloading syntax/theme assets)
//! let renderer = MarkdownRenderer::new();
//! let output = renderer.render("**bold** and `code`", 80);
//! assert!(!output.lines.is_empty());
//! ```

pub mod streaming;

use std::io::Write;

pub use leaf::inline::ResolvedFormat;
pub use leaf::markdown::{LinkSpan, ParseOutput, TocEntry};
pub use leaf::theme::{AppTheme, MarkdownTheme, ThemePreset};

pub use ratatui::style::{Color, Modifier, Style};
pub use ratatui::text::{Line, Span};

/// Reusable Markdown renderer that caches syntax and theme assets.
///
/// Preferred over one-shot functions when rendering multiple documents.
pub struct MarkdownRenderer {
    syntax_set: syntect::parsing::SyntaxSet,
    theme_set: syntect::highlighting::ThemeSet,
    preset: ThemePreset,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownRenderer {
    /// Create a renderer with the default OceanDark theme.
    pub fn new() -> Self {
        Self {
            syntax_set: syntect::parsing::SyntaxSet::load_defaults_newlines(),
            theme_set: syntect::highlighting::ThemeSet::load_defaults(),
            preset: ThemePreset::OceanDark,
        }
    }

    /// Create a renderer with a specific theme preset.
    pub fn with_theme(preset: ThemePreset) -> Self {
        Self {
            syntax_set: syntect::parsing::SyntaxSet::load_defaults_newlines(),
            theme_set: syntect::highlighting::ThemeSet::load_defaults(),
            preset,
        }
    }

    /// Change the theme preset.
    pub fn set_theme(&mut self, preset: ThemePreset) {
        self.preset = preset;
    }

    /// Current theme preset.
    pub fn theme(&self) -> ThemePreset {
        self.preset
    }

    /// Access the full app theme (UI + Markdown colors).
    pub fn app_theme(&self) -> &'static AppTheme {
        leaf::theme::theme_by_preset(self.preset)
    }

    /// Render Markdown source at the given column width.
    pub fn render(&self, source: &str, width: usize) -> ParseOutput {
        let app_theme = leaf::theme::theme_by_preset(self.preset);
        let syntect_theme = self.syntect_theme(&app_theme.syntax_theme_name);
        leaf::markdown::parse_with_width(
            source,
            &self.syntax_set,
            syntect_theme,
            width,
            &app_theme.markdown,
            false,
        )
    }

    /// Render a non-Markdown file as a syntax-highlighted code block.
    pub fn render_code_file(&self, source: &str, width: usize) -> ParseOutput {
        let app_theme = leaf::theme::theme_by_preset(self.preset);
        let syntect_theme = self.syntect_theme(&app_theme.syntax_theme_name);
        leaf::markdown::parse_with_width(
            source,
            &self.syntax_set,
            syntect_theme,
            width,
            &app_theme.markdown,
            true,
        )
    }

    /// Render Markdown to an ANSI-colored string.
    pub fn render_to_ansi(&self, source: &str, width: usize) -> String {
        let output = self.render(source, width);
        lines_to_ansi(&output.lines, width)
    }

    /// Render Markdown to plain text (no escape codes).
    pub fn render_to_plain(&self, source: &str, width: usize) -> String {
        let output = self.render(source, width);
        lines_to_plain(&output.lines, width)
    }

    fn syntect_theme<'a>(
        &'a self,
        name: &str,
    ) -> &'a syntect::highlighting::Theme {
        self.theme_set
            .themes
            .get(name)
            .or_else(|| self.theme_set.themes.get("base16-ocean.dark"))
            .or_else(|| self.theme_set.themes.values().next())
            .expect("syntect theme set is empty")
    }
}

// ── One-shot convenience functions ───────────────

/// Render Markdown source to an ANSI-colored string (one-shot, loads assets each call).
///
/// For repeated rendering, prefer [`MarkdownRenderer`].
pub fn render_to_ansi(source: &str, width: usize) -> String {
    MarkdownRenderer::new().render_to_ansi(source, width)
}

/// Render Markdown source to plain text (one-shot, loads assets each call).
pub fn render_to_plain(source: &str, width: usize) -> String {
    MarkdownRenderer::new().render_to_plain(source, width)
}

// ── Output conversion helpers ────────────────────

/// Convert ratatui `Line`s to an ANSI-colored string.
pub fn lines_to_ansi(lines: &[Line<'_>], width: usize) -> String {
    let mut buf = Vec::new();
    leaf::inline::write_lines(lines, ResolvedFormat::Ansi, width, &mut buf)
        .expect("write to Vec never fails");
    String::from_utf8(buf).expect("ANSI output is valid UTF-8")
}

/// Convert ratatui `Line`s to plain text.
pub fn lines_to_plain(lines: &[Line<'_>], width: usize) -> String {
    let mut buf = Vec::new();
    leaf::inline::write_lines(lines, ResolvedFormat::Plain, width, &mut buf)
        .expect("write to Vec never fails");
    String::from_utf8(buf).expect("plain output is valid UTF-8")
}

/// Write ratatui `Line`s to any writer in the given format.
pub fn write_lines<W: Write>(
    lines: &[Line<'_>],
    format: ResolvedFormat,
    width: usize,
    writer: &mut W,
) -> anyhow::Result<()> {
    leaf::inline::write_lines(lines, format, width, writer)
}

/// Available theme presets.
pub const THEME_PRESETS: [ThemePreset; 4] = [
    ThemePreset::Arctic,
    ThemePreset::Forest,
    ThemePreset::OceanDark,
    ThemePreset::SolarizedDark,
];
