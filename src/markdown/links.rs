use crate::theme::MarkdownTheme;
use ratatui::text::Line;

use super::width::display_width;
use super::LINK_MARKER;

/// A detected link within the rendered output, with its position and target URL.
#[non_exhaustive]
pub struct LinkSpan {
    /// Zero-based line index.
    pub line_idx: usize,
    /// Start column (display width).
    pub start_col: usize,
    /// End column (exclusive, display width).
    pub end_col: usize,
    /// Link target URL.
    pub url: String,
}

pub(super) fn build_link_spans(
    lines: &[Line<'_>],
    link_urls: &[String],
    theme: &MarkdownTheme,
) -> Vec<LinkSpan> {
    let mut spans = Vec::new();
    let mut url_idx = 0;
    for (line_idx, line) in lines.iter().enumerate() {
        if url_idx >= link_urls.len() {
            break;
        }
        let mut col = 0usize;
        let mut in_link = false;
        let mut link_start = 0usize;
        for span in &line.spans {
            let w = display_width(span.content.as_ref());
            if span.content.as_ref() == LINK_MARKER && span.style.fg == Some(theme.link_icon) {
                in_link = true;
                link_start = col;
            } else if in_link {
                let is_link_text = span.style.fg == Some(theme.link_text);
                if !is_link_text {
                    if col > link_start && url_idx < link_urls.len() {
                        spans.push(LinkSpan {
                            line_idx,
                            start_col: link_start,
                            end_col: col,
                            url: link_urls[url_idx].clone(),
                        });
                        url_idx += 1;
                    }
                    in_link = false;
                }
            }
            col += w;
        }
        if in_link && col > link_start && url_idx < link_urls.len() {
            spans.push(LinkSpan {
                line_idx,
                start_col: link_start,
                end_col: col,
                url: link_urls[url_idx].clone(),
            });
            url_idx += 1;
        }
    }
    spans
}
