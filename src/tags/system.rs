//! 系统操作标签处理器
//!
//! 实现 exec, save, load, debug, debugprint, caption, mouse, file, httpget, httppost 等系统操作标签。

use super::{ExecutionContext, TagHandler, TagResult};
use crate::error::Result;
use crate::event::Event;

/// [exec] 执行用户操作
pub struct ExecHandler;

impl TagHandler for ExecHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let command = ctx.instruction.get("command").unwrap_or("").to_string();
        let mode = ctx.instruction.get("mode")
            .and_then(|v| v.parse::<i32>().ok());
        Ok(TagResult::Emit(Event::Exec { command, mode }))
    }
}

/// [save] 存档
pub struct SaveHandler;

impl TagHandler for SaveHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let file = ctx.resolve_param("file")?.as_string();
        eprintln!("[TRACE] SaveHandler executed, file={file}");
        Ok(TagResult::Emit(Event::SaveGame { file }))
    }
}

/// [load] 读档
pub struct LoadHandler;

impl TagHandler for LoadHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let file = ctx.resolve_param("file")?.as_string();
        let trans_type = ctx.instruction.get("type")
            .and_then(|v| v.parse::<i32>().ok());
        Ok(TagResult::Emit(Event::LoadGame { file, trans_type }))
    }
}

/// [debug] 调试设置
pub struct DebugHandler;

impl TagHandler for DebugHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let mode = ctx.instruction.get("mode")
            .and_then(|v| v.parse::<i32>().ok());
        let level = ctx.instruction.get("level")
            .and_then(|v| v.parse::<i32>().ok());
        Ok(TagResult::Emit(Event::DebugConfig { mode, level }))
    }
}

/// [debugprint] 调试输出
pub struct DebugprintHandler;

impl TagHandler for DebugprintHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let level = ctx.instruction.get("level")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);
        let data = ctx.resolve_param("data")?.as_string();
        Ok(TagResult::Emit(Event::DebugPrint { level, data }))
    }
}

/// [debugreload] 调试重载
pub struct DebugreloadHandler;

impl TagHandler for DebugreloadHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Emit(Event::DebugReload))
    }
}

/// [caption] 窗口标题
pub struct CaptionHandler;

impl TagHandler for CaptionHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let data = ctx.resolve_param("data")?.as_string();
        Ok(TagResult::Emit(Event::Caption { data }))
    }
}

/// [mouse] 鼠标设置
pub struct MouseHandler;

impl TagHandler for MouseHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let left = ctx.instruction.get("left")
            .and_then(|v| v.parse::<i32>().ok());
        let top = ctx.instruction.get("top")
            .and_then(|v| v.parse::<i32>().ok());
        let hide = ctx.instruction.get("hide")
            .and_then(|v| v.parse::<i32>().ok());
        let autohide = ctx.instruction.get("autohide")
            .and_then(|v| v.parse::<u64>().ok());
        Ok(TagResult::Emit(Event::MouseConfig { left, top, hide, autohide }))
    }
}

/// [keyconfig] 按键配置
pub struct KeyconfigHandler;

impl TagHandler for KeyconfigHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let mut config = std::collections::HashMap::new();
        for (key, value) in &ctx.instruction.params {
            config.insert(key.clone(), value.clone());
        }
        Ok(TagResult::Emit(Event::KeyConfig(config)))
    }
}

/// [file] 文件操作
pub struct FileHandler;

impl TagHandler for FileHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let command = ctx.instruction.get("command").unwrap_or("").to_string();
        let src = ctx.instruction.get("src").map(String::from);
        let dst = ctx.instruction.get("dst").map(String::from);
        let target = ctx.instruction.get("target").map(String::from);
        Ok(TagResult::Emit(Event::FileOperation { command, src, dst, target }))
    }
}

/// [httpget] HTTP GET
pub struct HttpgetHandler;

impl TagHandler for HttpgetHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let url = ctx.resolve_param("url")?.as_string();
        Ok(TagResult::Emit(Event::HttpGet { url }))
    }
}

/// [httppost] HTTP POST
pub struct HttppostHandler;

impl TagHandler for HttppostHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let url = ctx.resolve_param("url")?.as_string();
        let mut params = std::collections::HashMap::new();
        for (key, value) in &ctx.instruction.params {
            if key != "url" {
                params.insert(key.clone(), value.clone());
            }
        }
        Ok(TagResult::Emit(Event::HttpPost { url, params }))
    }
}

/// [openbrowser] 打开浏览器
pub struct OpenbrowserHandler;

impl TagHandler for OpenbrowserHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let url = ctx.resolve_param("url")?.as_string();
        Ok(TagResult::Emit(Event::OpenBrowser { url }))
    }
}

/// [autosave] 自动存档
pub struct AutosaveHandler;

impl TagHandler for AutosaveHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let allow = ctx.instruction.get("allow")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1) != 0;
        Ok(TagResult::Emit(Event::AutoSaveConfig { allow }))
    }
}

/// [avoid] 紧急回避
pub struct AvoidHandler;

impl TagHandler for AvoidHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let allow = ctx.instruction.get("allow")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1) != 0;
        Ok(TagResult::Emit(Event::AvoidConfig { allow }))
    }
}

/// [vibrate] 振动
pub struct VibrateHandler;

impl TagHandler for VibrateHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let time = ctx.instruction.get("time")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        Ok(TagResult::Emit(Event::Vibrate { time }))
    }
}

/// [statusbar] 状态栏
pub struct StatusbarHandler;

impl TagHandler for StatusbarHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let visible = ctx.instruction.get("visible")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1) != 0;
        Ok(TagResult::Emit(Event::StatusBar { visible }))
    }
}

/// [purchase] 应用内购买
pub struct PurchaseHandler;

impl TagHandler for PurchaseHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let item = ctx.resolve_param("item")?.as_string();
        Ok(TagResult::Emit(Event::Purchase { item }))
    }
}

/// [callnative] 调用原生代码
pub struct CallnativeHandler;

impl TagHandler for CallnativeHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let function = ctx.instruction.get("function").unwrap_or("").to_string();
        let mut params = std::collections::HashMap::new();
        for (key, value) in &ctx.instruction.params {
            if key != "function" {
                params.insert(key.clone(), value.clone());
            }
        }
        Ok(TagResult::Emit(Event::CallNative { function, params }))
    }
}
