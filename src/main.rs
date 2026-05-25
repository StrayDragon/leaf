use anyhow::{Context, Result};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{fs::OpenOptions, io, io::IsTerminal, io::Write, path::PathBuf};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

use leaf::app::{App, AppConfig};
use leaf::cli::{parse_cli, print_usage, print_version, CliOptions};
use leaf::markdown::{hash_str, parse_markdown, parse_markdown_with_width, read_file_state};
use leaf::runtime::run;
use leaf::terminal::{finish_with_restore, TerminalSession};
use leaf::theme::{
    app_theme, current_syntect_theme, resolve_theme_selection, set_theme_selection,
    validate_theme_syntax,
};
use leaf::update::run_update;
use leaf::{read_stdin_limited, MAX_STDIN_BYTES};

fn resolve_configured_width(
    cli_width: Option<usize>,
    config_width: Option<usize>,
) -> Option<usize> {
    if let Some(w) = cli_width {
        return Some(w);
    }
    if let Ok(val) = std::env::var("LEAF_WIDTH") {
        if let Ok(w) = val.parse::<usize>() {
            if w >= 20 {
                return Some(w);
            }
        }
    }
    config_width.map(|w| w.max(20))
}

fn append_config_warning(warning: &mut Option<String>, next: Option<String>) {
    let Some(next) = next else {
        return;
    };
    match warning {
        Some(existing) => {
            existing.push_str("; ");
            existing.push_str(&next);
        }
        None => *warning = Some(next),
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let options = parse_cli(&args)?;

    if options.print_help {
        print_usage();
        return Ok(());
    }
    if options.print_version {
        print_version();
        return Ok(());
    }
    if options.update {
        run_update()?;
        return Ok(());
    }
    if options.config {
        leaf::config::run_config()?;
        return Ok(());
    }
    if let Some(ref ac_arg) = options.auto_complete {
        leaf::completions::run_auto_complete(ac_arg)?;
        return Ok(());
    }
    let CliOptions {
        picker,
        watch: watch_from_cli,
        debug_input,
        file_arg,
        theme: cli_theme,
        editor: cli_editor,
        inline: mut inline_spec,
        width: cli_width,
        ..
    } = options;

    let overrides = leaf::config::CliOverrides {
        width: cli_width,
        theme: cli_theme.clone(),
    };
    let (user_config, mut config_warning) = leaf::config::load_config(&overrides);

    let theme_selection = if let Some(theme_name) = cli_theme.as_deref() {
        resolve_theme_selection(theme_name, &user_config.themes, None)
            .map_err(|message| anyhow::anyhow!("{message}"))?
    } else if let Some(theme_name) = std::env::var("LEAF_THEME")
        .ok()
        .filter(|s| !s.is_empty())
        .as_deref()
    {
        resolve_theme_selection(theme_name, &user_config.themes, None).unwrap_or_default()
    } else if let Some(theme_name) = user_config.theme.as_deref() {
        resolve_theme_selection(
            theme_name,
            &user_config.themes,
            user_config.config_dir.as_deref(),
        )
        .unwrap_or_default()
    } else {
        leaf::theme::ThemeSelection::default()
    };

    let watch_from_config = user_config.watch.unwrap_or(false);
    let max_width = resolve_configured_width(cli_width, user_config.width);

    if let Some(ref mut spec) = inline_spec {
        if spec.width.is_none() {
            spec.width = max_width;
        }
    }

    let resolved_editor =
        leaf::editor::resolve_editor(cli_editor.as_deref(), user_config.editor.as_deref());
    leaf::runtime::debug_log(debug_input, &format!("main start args={args:?}"));

    if debug_input {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("leaf-debug.log")
            .context("Cannot create leaf-debug.log")?;
        writeln!(file, "leaf debug input log").ok();
    }

    let mut open_browser_picker_dir = None;
    let mut open_fuzzy_picker_dir = None;
    let mut dir_arg = None;
    let (src, filename, filepath) = if let Some(f) = file_arg {
        let path = PathBuf::from(&f);
        if path.is_dir() {
            let label = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string());
            if picker {
                open_browser_picker_dir = Some(path.clone());
            } else {
                open_fuzzy_picker_dir = Some(path.clone());
            }
            dir_arg = Some(path);
            (String::new(), label, None)
        } else if picker {
            anyhow::bail!("--picker cannot be combined with a file path");
        } else {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Cannot read: {}", path.display()))?;
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or(f);
            (content, name, Some(path))
        }
    } else {
        if io::stdin().is_terminal() {
            let cwd = std::env::current_dir().context("Cannot read current directory")?;
            let label = cwd
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| cwd.display().to_string());
            if picker {
                open_browser_picker_dir = Some(cwd);
            } else {
                open_fuzzy_picker_dir = Some(cwd);
            }
            (String::new(), label, None)
        } else {
            if watch_from_cli {
                eprintln!("Error: --watch requires a file path (stdin cannot be watched)");
                std::process::exit(1);
            }
            let mut stdin = io::stdin().lock();
            let buf = read_stdin_limited(&mut stdin, MAX_STDIN_BYTES)?;
            (buf, "stdin".to_string(), None)
        }
    };

    let is_file_input = filepath.is_some();
    let watch = watch_from_cli || (watch_from_config && is_file_input);

    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    append_config_warning(
        &mut config_warning,
        validate_theme_syntax(&theme_selection, &ts),
    );
    set_theme_selection(theme_selection);
    let theme = current_syntect_theme(&ts).clone();
    leaf::runtime::debug_log(
        debug_input,
        &format!(
            "main input_ready filename={filename} filepath={} picker={} watch={}",
            filepath
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<none>".to_string()),
            picker,
            watch
        ),
    );

    let last_file_state = filepath.as_ref().and_then(read_file_state);
    let last_content_hash = hash_str(&src);

    let ext = filepath
        .as_ref()
        .and_then(|p| p.extension())
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let (src, file_mode) = App::wrap_as_code_block(src, ext, &ss);

    if let Some(ref spec) = inline_spec {
        if src.is_empty() && filepath.is_none() {
            anyhow::bail!("--inline requires a file path or stdin input");
        }

        let is_tty = io::stdout().is_terminal();
        let width = leaf::inline::render_width(spec, is_tty);
        let format = leaf::inline::resolve_format(spec, is_tty);

        let at = app_theme();
        let (mut lines, _, _) =
            parse_markdown_with_width(&src, &ss, &theme, width, &at.markdown, file_mode);

        while lines.last().is_some_and(|l| {
            l.spans.is_empty() || l.spans.iter().all(|s| s.content.trim().is_empty())
        }) {
            lines.pop();
        }

        let stdout = io::stdout();
        let mut writer = io::BufWriter::new(stdout.lock());
        leaf::inline::write_lines(&lines, format, width, &mut writer)?;
        return Ok(());
    }

    let at = app_theme();
    let (lines, toc, link_spans) = parse_markdown(&src, &ss, &theme, &at.markdown, file_mode);
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename,
            source: src,
            debug_input,
            watch,
            filepath,
            last_file_state,
        },
    );
    app.set_link_spans(link_spans);
    app.set_last_content_hash(last_content_hash);
    app.set_watch_from_config(watch_from_config);
    app.set_max_width(max_width);
    app.set_extras(user_config.extras);
    app.set_file_mode(file_mode);
    app.set_editor_config(Some(resolved_editor));
    app.set_config_warning(config_warning);
    if let Some(dir) = dir_arg {
        app.set_dir_arg(dir);
    }
    if let Some(dir) = open_browser_picker_dir {
        app.queue_file_picker(dir);
    }
    if let Some(dir) = open_fuzzy_picker_dir {
        app.queue_fuzzy_file_picker(dir);
    }
    leaf::runtime::debug_log(
        debug_input,
        &format!(
            "main app_ready pending_picker={} picker_loading={}",
            app.has_pending_picker(),
            app.is_picker_loading()
        ),
    );

    let mut stdout = io::stdout();
    print!("\x1b]0;leaf\x07");
    let _ = io::stdout().flush();
    leaf::runtime::debug_log(debug_input, "terminal enter start");
    let mut session = TerminalSession::enter(&mut stdout)?;
    leaf::runtime::debug_log(debug_input, "terminal enter done");
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    leaf::runtime::debug_log(debug_input, "terminal new done");
    terminal.clear()?;
    leaf::runtime::debug_log(debug_input, "terminal clear done");
    let initial_draw_result = (|| -> Result<()> {
        let area = terminal.size()?;
        leaf::runtime::debug_log(
            debug_input,
            &format!(
                "initial_draw size width={} height={}",
                area.width, area.height
            ),
        );
        leaf::runtime::prepare_initial_picker_state(area.width as usize, &mut app, &ss, &ts)?;
        leaf::runtime::debug_log(debug_input, "initial_draw draw start");
        terminal.draw(|f| leaf::render::ui(f, &mut app))?;
        leaf::runtime::debug_log(debug_input, "initial_draw draw done");
        session.finish_initial_draw(&mut terminal)?;
        leaf::runtime::debug_log(debug_input, "initial_draw sync end done");
        Ok(())
    })();
    let run_result = match initial_draw_result {
        Ok(()) => {
            leaf::runtime::debug_log(debug_input, "run loop start");
            run(&mut terminal, &mut app, &ss, &ts, true)
        }
        Err(err) => Err(err),
    };
    leaf::runtime::debug_log(debug_input, "run loop end");
    let restore_result = session.restore(&mut terminal);
    finish_with_restore(run_result, restore_result)
}
