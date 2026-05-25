//! # leaf — Terminal Markdown rendering library
//!
//! `leaf` provides a Markdown-to-terminal rendering pipeline built on
//! `pulldown-cmark` for parsing and [`ratatui`] for styled output.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use leaf::markdown;
//! use leaf::theme::{theme_by_preset, ThemePreset};
//! use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};
//!
//! let ss = SyntaxSet::load_defaults_newlines();
//! let ts = ThemeSet::load_defaults();
//! let app_theme = theme_by_preset(ThemePreset::OceanDark);
//! let syntect_theme = &ts.themes["base16-ocean.dark"];
//!
//! let output = markdown::parse(
//!     "# Hello\nworld", &ss, syntect_theme, &app_theme.markdown, false,
//! );
//! assert!(!output.lines.is_empty());
//! ```

use anyhow::{bail, Context, Result};
use std::io::Read;

// ── Public API (Tier 1) ──────────────────────────

/// Markdown parsing pipeline: source text → styled [`ratatui::text::Line`]s.
pub mod markdown;

/// Non-interactive stdout rendering (ANSI / plain text).
pub mod inline;

/// Color themes: built-in presets and custom TOML theme loading.
pub mod theme;

pub use markdown::ParseOutput;

// ── Semi-public API (Tier 2) ─────────────────────

/// Configuration file loading and defaults.
pub mod config;

/// External editor detection and launch.
pub mod editor;

// ── Internal modules (used by binary, hidden from docs) ──
#[doc(hidden)]
pub mod app;
#[doc(hidden)]
pub mod cli;
#[doc(hidden)]
pub mod clipboard;
#[doc(hidden)]
pub mod completions;
#[doc(hidden)]
pub mod render;
#[doc(hidden)]
pub mod runtime;
#[doc(hidden)]
pub mod terminal;
#[doc(hidden)]
pub mod update;

#[cfg(test)]
mod tests;

/// Maximum bytes accepted from stdin (8 MiB).
pub const MAX_STDIN_BYTES: usize = 8 * 1024 * 1024;

/// Read up to `max_bytes` from a reader, returning an error if the limit is exceeded
/// or the content is not valid UTF-8.
pub fn read_stdin_limited<R: Read>(reader: &mut R, max_bytes: usize) -> Result<String> {
    let mut buf = Vec::with_capacity(max_bytes.min(8192));
    let limit = u64::try_from(max_bytes)
        .ok()
        .and_then(|value| value.checked_add(1))
        .context("stdin size limit is too large")?;
    reader
        .take(limit)
        .read_to_end(&mut buf)
        .context("Cannot read stdin")?;
    if buf.len() > max_bytes {
        bail!(
            "stdin exceeds the maximum supported size of {} bytes",
            max_bytes
        );
    }
    String::from_utf8(buf).context("stdin is not valid UTF-8")
}

#[cfg(test)]
pub(crate) use config::{config_path, LeafConfig};
#[cfg(test)]
pub(crate) use editor::{
    binary_name, classify, resolve_editor, split_editor_cmd, try_new_tab_command, EditorKind,
    TerminalEmulator,
};
#[cfg(test)]
pub(crate) use markdown::toc::{
    normalize_toc, should_hide_single_h1, should_promote_h2_when_no_h1, toc_display_level, TocEntry,
};
#[cfg(test)]
pub(crate) use markdown::{display_width, line_plain_text};
#[cfg(test)]
pub(crate) use read_stdin_limited as read_stdin_with_limit;
#[cfg(test)]
pub(crate) use render::wrap_path_lines;
#[cfg(test)]
pub(crate) use runtime::should_handle_key;
#[cfg(test)]
pub(crate) use theme::{
    app_theme, parse_theme_color, parse_theme_preset, theme_preset_label, CustomThemeConfig,
    ThemePreset, ThemeSelection, THEME_PRESETS,
};
#[cfg(test)]
pub(crate) use update::{
    asset_name_for_target, expected_asset_download_url, find_expected_checksum, is_newer_version,
    validate_download_size, validate_sha256_hex,
};
