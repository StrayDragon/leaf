# Fork leaf 暴露 Library API 指南

## 当前状态

> **已完成 fork 拆分。** `feat/expose-lib-api` 分支已将 leaf 从 binary-only 拆分为 lib + bin。
> 所有 227 个测试通过，xylitol 可通过 `leaf = { path = "../leaf" }` 直接引用。

原始状态：binary-only crate，无 `lib.rs`，~482 个 `pub(crate)` 声明。
当前状态：`src/lib.rs` 暴露 Tier 1 公开 API，`main.rs` 为薄二进制包装。

## 推荐的公开 API 分层

### Tier 1 — 最小库 API（3-5 人天）

对应 `--inline` 工作流，开箱即用：

```rust
// leaf::markdown
pub fn parse_markdown(src, ss, theme, md_theme, file_mode) -> ParseOutput;
pub fn parse_markdown_with_width(src, ss, theme, width, md_theme, file_mode) -> ParseOutput;

pub struct ParseOutput {
    pub lines: Vec<Line<'static>>,   // ratatui styled lines
    pub toc: Vec<TocEntry>,
    pub links: Vec<LinkSpan>,
}

// leaf::inline
pub fn write_lines<W: Write>(lines, format, max_width, writer) -> Result<()>;
pub fn parse_inline_spec(s: &str) -> Result<InlineSpec>;

// leaf::theme
pub fn theme_by_preset(preset: ThemePreset) -> &'static AppTheme;
pub fn resolve_theme_selection(name, custom_themes, base_dir) -> Result<ThemeSelection>;
```

### Tier 2 — 扩展 API

- `leaf::config` — `LeafConfig`, `load_config`, `config_path`
- `leaf::editor` — `resolve_editor`, `EditorKind`
- `leaf::highlight` — `highlight_line` 搜索高亮

### Tier 3 — 完整 TUI 嵌入（1-2 周）

- `leaf::app::App` + `leaf::render::ui` + `leaf::runtime::run`
- 仅在需要将 leaf TUI 嵌入其他应用时才考虑

## 需要公开的核心类型

| 类型 | 位置 | 用途 |
|------|------|------|
| `MarkdownTheme` | `theme/mod.rs` | 40+ 颜色字段，渲染样式输入 |
| `AppTheme` | `theme/mod.rs` | 包装 UiTheme + MarkdownTheme + syntect 名称 |
| `ThemePreset` | `theme/mod.rs` | Arctic / Forest / OceanDark / SolarizedDark |
| `ThemeSelection` | `theme/mod.rs` | 预设或自定义 |
| `TocEntry` | `markdown/toc.rs` | 目录条目（level, title, line） |
| `LinkSpan` | `markdown/links.rs` | 链接位置（line_idx, col range, url） |
| `InlineSpec` | `inline.rs` | 内联渲染规格 |
| `ResolvedFormat` | `inline.rs` | Ansi / Plain |

**硬依赖暴露:** 公开 API 不可避免暴露 `ratatui::text::Line` 和 `syntect` 类型。

## 必要的重构步骤

### Step 1: 创建 `src/lib.rs`（结构拆分）

将所有 `mod` 声明从 `main.rs` 移到 `lib.rs`：

```rust
// src/lib.rs
pub mod markdown;
pub mod inline;
pub mod theme;

// 仅 TUI 功能时编译
#[cfg(feature = "tui")]
pub(crate) mod app;
#[cfg(feature = "tui")]
pub(crate) mod render;
#[cfg(feature = "tui")]
pub(crate) mod runtime;

pub(crate) mod terminal;
pub(crate) mod editor;
pub(crate) mod config;
```

`main.rs` 简化为：

```rust
use leaf::{...};
fn main() -> Result<()> { /* 原有逻辑 */ }
```

**工作量:** 低（半天），主要是机械性的路径修改。

### Step 2: Cargo.toml 变更

```toml
[lib]
name = "leaf"       # fork 建议用 leaf-core / leaf-lib / leafmd
path = "src/lib.rs"

[[bin]]
name = "leaf"
path = "src/main.rs"

[features]
default = ["tui"]
tui = []              # 控制 app/runtime/render 模块
update = ["reqwest", "sha2", "semver"]  # 可选的自更新

[dependencies]
reqwest = { version = "0.12", optional = true, ... }
sha2   = { version = "0.10", optional = true }
semver = { version = "1.0", optional = true }
```

### Step 3: 修复架构耦合

| 问题 | 位置 | 修复方案 | 工作量 |
|------|------|----------|--------|
| **全局主题单例** | `theme/mod.rs` `CURRENT_THEME` | 库 API 要求显式传递 `&MarkdownTheme`；保留 `set_theme_selection` 供 binary 使用 | 中（1-2 天） |
| **`TableBuf::render` 隐式使用 `app_theme()`** | `markdown/tables.rs:329` | 添加 `md_theme: &MarkdownTheme` 参数 | 低（1 小时） |
| **`read_file_state` → `app::FileState`** | `markdown/mod.rs` | 将 `FileState` 移到 `leaf::io` 或 `leaf::watch` | 低（2 小时） |
| **`wrap_as_code_block` 在 `App` 上** | `app/io_picker.rs` | 移到 `leaf::input` 或 `markdown` | 低（2 小时） |
| **`LinkSpan` 字段私有** | `markdown/links.rs` | 字段改为 `pub` 或添加访问器 | 低（30 分钟） |

### Step 4: 测试迁移

- 将 `#[cfg(test)] mod tests` 移到 `lib.rs`
- `use crate::*` 改为 `use leaf::{...}`
- 或转为 `tests/*.rs` 集成测试

**工作量:** 中（1 天）

### Step 5: API 稳定性

- 用 `ParseOutput` 结构体替代裸 tuple 返回值
- 枚举加 `#[non_exhaustive]`
- 添加 `rustdoc` 示例
- 定义 semver 策略

## 模块适配性评估

### 可直接暴露（仅需改 visibility）

| 模块 | 理由 |
|------|------|
| `inline` | 干净自包含，驱动 `--inline` 模式 |
| `markdown` 核心 | 主要价值点，已参数化 |
| `markdown/width` | 纯文本宽度工具 |
| `markdown/toc` | 纯 TOC 规范化逻辑 |
| `markdown/latex` | 单函数 `to_unicode` |
| `theme` 类型 + 预设 | 静态数据 + 解析逻辑 |

### 不适合初始公开

| 模块 | 原因 |
|------|------|
| `app` | 2000+ 行 TUI 状态，100+ 方法 |
| `runtime` | crossterm 事件循环绑定 App |
| `render` | `ui(f, &mut App)` 无法脱离 App 使用 |
| `update` | 二进制自更新，太特殊 |
| `completions` | Shell 补全安装器 |

## 快速路径

**最小可行 fork:**

1. 添加 `lib.rs` + `Cargo.toml` `[lib]`
2. 将 Tier 1 类型/函数改为 `pub`
3. 修复 `tables.rs` 全局主题 + `FileState` 耦合
4. 发布

**延后到有消费者需求时再做:** 全局主题重构、TUI 嵌入、Cargo features

## 消费者使用示例

```rust
use leaf::{
    markdown,
    inline::{write_lines, ResolvedFormat},
    theme::{theme_by_preset, ThemePreset},
};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

fn render_to_ansi(markdown_source: &str, width: usize) -> String {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let app_theme = theme_by_preset(ThemePreset::OceanDark);
    let syntect_theme = &ts.themes["base16-ocean.dark"];

    // parse() returns ParseOutput { lines, toc, links }
    let output = markdown::parse_with_width(
        markdown_source, &ss, syntect_theme, width, &app_theme.markdown, false,
    );

    let mut buf = Vec::new();
    write_lines(&output.lines, ResolvedFormat::Ansi, width, &mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

// Add to Cargo.toml:
// leaf = { path = "../leaf", default-features = false }
// — or with git:
// leaf = { git = "https://github.com/StrayDragon/leaf", branch = "feat/expose-lib-api" }
```

## 命名建议

上游 `leaf` 在 crates.io 可能冲突，fork 建议：
- `leaf-core` — 强调核心渲染
- `leaf-md` / `leafmd` — 强调 Markdown 功能
- `leaf-render` — 强调渲染管线
