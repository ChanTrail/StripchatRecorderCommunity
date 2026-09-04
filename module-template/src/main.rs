//! 社区模块模板 / Community Module Template
//!
//! **需要修改的地方（仅这三处）：**
//! 1. `Cargo.toml` 中的 `name` 和 `[[bin]].name` — 改为你的模块 ID
//! 2. `DESCRIBE` 常量中的 `description`、`params`、`i18n` — 描述你的模块功能和参数
//! 3. `run()` 函数中的处理逻辑
//!
//! `id`、`name`（显示名）、`version` 均在运行时从 `Cargo.toml` 自动读取，无需手动填写。
//!
//! **What to change (only these three things):**
//! 1. `name` and `[[bin]].name` in `Cargo.toml` — set to your module ID
//! 2. `description`, `params`, `i18n` in the `DESCRIBE` constant — describe your module
//! 3. Processing logic in the `run()` function
//!
//! `id`, display `name`, and `version` are all read from `Cargo.toml` at runtime automatically.
//!
//! ## 协议概述 / Protocol Overview
//!
//! StripchatRecorder 通过以下方式调用模块：
//! StripchatRecorder invokes modules as follows:
//!
//! 1. **describe 模式**：`./my_module --describe`
//!    向 stdout 打印 JSON 元数据后退出。
//!    Print JSON metadata to stdout and exit.
//!
//! 2. **执行模式**：将 JSON 输入写入 stdin，读取 stdout 输出。
//!    Write JSON input to stdin; read stdout for progress and result.
//!
//! ### stdin 输入格式 / stdin input format
//! ```json
//! {
//!   "inputs": ["/path/to/input/file_or_dir"],
//!   "params": { "my_param": "value" },
//!   "exe_dir": "/path/to/backend/dir",
//!   "max_tmp_mb": 51200,
//!   "recording": {
//!     "video_path": "/path/to/video.mp4",
//!     "started_at": "2026-09-04T12:00:00+08:00",
//!     "username": "alice"
//!   }
//! }
//! ```
//!
//! ### stdout 输出格式 / stdout output format
//! - `PROGRESS:{done}/{total}` — 进度上报，total 固定为 10000 / Progress, total is always 10000
//! - `STATUS:{text}` — 可选状态文字 / Optional status text
//! - 最后一行 JSON（模块返回值）/ Final JSON line (module return value):
//!   `{"code":"ok","message":"...","outputs":["/path/to/output"]}`
//!
//! ### 结果码 / Result codes
//! - `"ok"`        — 成功，outputs 传给下游节点 / Success, outputs forwarded to next node
//! - `"done"`      — 成功，无输出，流水线在此终止 / Success, no output, pipeline ends here
//! - `"skipped"`   — 已跳过（如输出已存在）/ Skipped (e.g. output already exists)
//! - `"error"`     — 失败 / Failure

use serde::Deserialize;
use std::io::{self, Read};
use std::path::PathBuf;

// ─── 模块元数据模板 / Module Metadata Template ────────────────────────────────
//
// ✏️  修改这里：description、params（参数定义）、i18n（多语言翻译）
// ✏️  Edit here: description, params, and i18n translations
//
// 不需要填写的字段（运行时自动注入）：
// Fields you do NOT need to fill in (auto-injected at runtime):
//   "id"      ← env!("CARGO_PKG_NAME")  即 Cargo.toml 的 name
//   "name"    ← id 的 Title Case 形式，如 "my_module" → "My Module"
//   "version" ← env!("CARGO_PKG_VERSION")  即 Cargo.toml 的 version
const DESCRIBE: &str = r#"{
  "description": "Does something useful with the recording",
  "inputTypes": ["video_file"],
  "outputTypes": ["video_file"],
  "official": false,
  "params": [
    {
      "key": "my_param",
      "label": "My Parameter",
      "type": "string",
      "default": ""
    }
  ],
  "i18n": {
    "zh-CN": {
      "description": "对录制文件做一些处理",
      "params": {
        "my_param": { "label": "我的参数" }
      }
    }
  }
}"#;

// ─── 进度常量 / Progress Constant ─────────────────────────────────────────────

/// 总进度刻度（与主程序约定固定为 10000）。
/// Total progress scale (fixed at 10000 by protocol).
const PROGRESS_SCALE: u32 = 10_000;

fn emit_progress(done: u32, total: u32) {
    if total == 0 {
        return;
    }
    let scaled = ((done as u64 * PROGRESS_SCALE as u64) / total as u64).min(PROGRESS_SCALE as u64);
    println!("PROGRESS:{}/{}", scaled, PROGRESS_SCALE);
}

// ─── stdin 输入结构 / stdin Input Struct ──────────────────────────────────────

#[derive(Debug, Deserialize)]
struct Input {
    inputs: Vec<String>,
    #[serde(default)]
    params: serde_json::Value,
    /// 后端可执行文件目录，可用于定位 tmp/ 等运行时目录
    /// Backend executable directory, useful for locating tmp/ and other runtime dirs
    #[serde(default)]
    exe_dir: String,
    /// 临时目录最大占用（MB）/ Max tmp dir size (MB)
    #[serde(default)]
    max_tmp_mb: u64,
    /// 录制上下文（视频路径、开始时间、主播用户名）
    /// Recording context (video path, start time, streamer username)
    #[serde(default)]
    recording: Option<serde_json::Value>,
}

impl Input {
    /// 读取指定参数值（字符串），不存在时返回默认值。
    /// Read a parameter value (string), returning the default if absent.
    fn param(&self, key: &str, default: &str) -> String {
        self.params
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or(default)
            .to_string()
    }

    /// 返回临时目录路径（exe_dir/tmp/），目录自动创建。
    /// Returns the temp directory path (exe_dir/tmp/), creating it if needed.
    fn tmp_dir(&self) -> PathBuf {
        let base = if self.exe_dir.is_empty() {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."))
        } else {
            PathBuf::from(&self.exe_dir)
        };
        let tmp = base.join("tmp");
        let _ = std::fs::create_dir_all(&tmp);
        tmp
    }
}

// ─── 主处理逻辑 / Main Processing Logic ──────────────────────────────────────

/// 模块主逻辑：从 stdin JSON 读取输入，处理后向 stdout 输出结果 JSON。
/// Main module logic: reads input from stdin JSON, processes it, outputs result JSON to stdout.
fn run() -> serde_json::Value {
    // 1. 读取并解析 stdin JSON / Read and parse stdin JSON
    let mut buf = String::new();
    io::stdin().lock().read_to_string(&mut buf).ok();

    let input: Input = match serde_json::from_str(buf.trim()) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!({
                "code": "error",
                "message": format!("Failed to parse stdin: {}", e),
                "outputs": []
            });
        }
    };

    // 2. 取第一个输入路径 / Get first input path
    let input_path = match input.inputs.first() {
        Some(p) => PathBuf::from(p),
        None => {
            return serde_json::json!({
                "code": "error",
                "message": "inputs[0] is required",
                "outputs": []
            });
        }
    };

    if !input_path.exists() {
        return serde_json::json!({
            "code": "error",
            "message": format!("Input not found: {}", input_path.display()),
            "outputs": []
        });
    }

    // 3. 读取参数 / Read parameters
    let my_param = input.param("my_param", "");

    emit_progress(0, 3);

    // ── ✏️  在这里实现你的处理逻辑 ──────────────────────────────────────────
    // ── ✏️  Implement your processing logic here ─────────────────────────────
    eprintln!(
        "[{}] input={}, my_param={}, tmp={}",
        env!("CARGO_PKG_NAME"),
        input_path.display(),
        my_param,
        input.tmp_dir().display()
    );

    emit_progress(1, 3);

    // 模拟处理 / Simulate processing
    // std::thread::sleep(std::time::Duration::from_secs(1));

    emit_progress(3, 3);
    // ── 处理逻辑结束 / End of processing logic ───────────────────────────────

    // 4. 返回结果 / Return result
    //    "ok" + outputs  → outputs[0] 传给下游节点 / forwarded to next node
    //    "done"          → 流水线在此终止（无输出）/ pipeline ends here (no output)
    //    "skipped"       → 跳过，仍提供 outputs / skipped, still provide outputs
    //    "error"         → 失败，流水线中止 / failure, pipeline aborts
    serde_json::json!({
        "code": "ok",
        "message": "processed successfully",
        "outputs": [input_path.to_string_lossy()]
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("--describe") {
        // describe 模式：将 id、name（Title Case）、version 注入 DESCRIBE JSON 后输出
        // Describe mode: inject id, display name (Title Case), and version into DESCRIBE JSON
        let pkg_name    = env!("CARGO_PKG_NAME");
        let pkg_version = env!("CARGO_PKG_VERSION");

        // Title Case：下划线替换为空格，每词首字母大写
        // Title Case: replace underscores with spaces, capitalize first letter of each word
        let display_name: String = pkg_name
            .split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        let mut desc: serde_json::Value =
            serde_json::from_str(DESCRIBE).expect("DESCRIBE is valid JSON");
        desc["id"]      = serde_json::Value::String(pkg_name.to_string());
        desc["name"]    = serde_json::Value::String(display_name);
        desc["version"] = serde_json::Value::String(pkg_version.to_string());
        print!("{}", desc);
        return;
    }

    let result = run();
    println!("{}", result);
}
