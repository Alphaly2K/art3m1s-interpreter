//! 剧情脚本标签处理器
//!
//! 实现 print, rt, rp, font, ruby, link, glyph, scetween 等剧情文本相关标签。

use super::{ExecutionContext, TagHandler, TagResult};
use crate::error::Result;
use crate::event::Event;
use std::collections::HashMap;

/// [print] 显示场景文本
pub struct PrintHandler;

impl TagHandler for PrintHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let data = ctx.instruction.get("data").unwrap_or("");
        let value = ctx.evaluator().resolve_param(data)?;
        Ok(TagResult::Emit(Event::ScenarioText {
            content: value.as_string(),
            inline: false,
        }))
    }
}

/// [rt] 场景文本换行
pub struct RtHandler;

impl TagHandler for RtHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Emit(Event::LineBreak))
    }
}

/// [rp] 场景文本分页
pub struct RpHandler;

impl TagHandler for RpHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let backlog = ctx.instruction.get("backlog")
            .and_then(|v| v.parse::<i32>().ok());
        Ok(TagResult::Emit(Event::PageBreak { backlog }))
    }
}

/// [font] 字体设置
pub struct FontHandler;

impl TagHandler for FontHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let mut settings = HashMap::new();
        for (key, value) in &ctx.instruction.params {
            let resolved = ctx.evaluator().resolve_param(value)?;
            settings.insert(key.clone(), resolved.as_string());
        }
        Ok(TagResult::Emit(Event::FontSettings(settings)))
    }
}

/// [font_close] 回退字体设置
pub struct FontCloseHandler;

impl TagHandler for FontCloseHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Emit(Event::FontClose))
    }
}

/// [fontdefault] 默认字体设置
pub struct FontDefaultHandler;

impl TagHandler for FontDefaultHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let mut settings = HashMap::new();
        for (key, value) in &ctx.instruction.params {
            let resolved = ctx.evaluator().resolve_param(value)?;
            settings.insert(key.clone(), resolved.as_string());
        }
        Ok(TagResult::Emit(Event::FontDefault(settings)))
    }
}

/// [fontinit] 初始化字体为默认
pub struct FontInitHandler;

impl TagHandler for FontInitHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Emit(Event::FontInit))
    }
}

/// [ruby] 开始注音
pub struct RubyHandler;

impl TagHandler for RubyHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let text = ctx.instruction.get("text").unwrap_or("").to_string();
        Ok(TagResult::Emit(Event::RubyStart { text }))
    }
}

/// [/ruby] 结束注音
pub struct RubyEndHandler;

impl TagHandler for RubyEndHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Emit(Event::RubyEnd))
    }
}

/// [link] 开始链接
pub struct LinkHandler;

impl TagHandler for LinkHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let file = ctx.instruction.get("file").map(String::from);
        let label = ctx.instruction.get("label").map(String::from);
        let link_type = ctx.instruction.get("type")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);
        let color = ctx.instruction.get("color").map(String::from);
        Ok(TagResult::Emit(Event::LinkStart {
            file,
            label,
            link_type,
            color,
        }))
    }
}

/// [/link] 结束链接
pub struct LinkEndHandler;

impl TagHandler for LinkEndHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Emit(Event::LinkEnd))
    }
}

/// [linkdisable] 禁用链接
pub struct LinkDisableHandler;

impl TagHandler for LinkDisableHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Emit(Event::LinkDisable))
    }
}

/// [linkenable] 启用链接
pub struct LinkEnableHandler;

impl TagHandler for LinkEnableHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Emit(Event::LinkEnable))
    }
}

/// [glyph] 点击等待图标设置
pub struct GlyphHandler;

impl TagHandler for GlyphHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let mut settings = HashMap::new();
        for (key, value) in &ctx.instruction.params {
            settings.insert(key.clone(), value.clone());
        }
        Ok(TagResult::Emit(Event::GlyphConfig(settings)))
    }
}

/// [chgmsg] 切换消息层
pub struct ChgmsgHandler;

impl TagHandler for ChgmsgHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.instruction.get("id").map(String::from);
        let layered = ctx.instruction.get("layered")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);
        Ok(TagResult::Emit(Event::MessageLayerSwitch { id, layered }))
    }
}

/// [chgmsg_close] 回退消息层
pub struct ChgmsgCloseHandler;

impl TagHandler for ChgmsgCloseHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Emit(Event::MessageLayerPop))
    }
}

/// [scetween] 文本动画
pub struct ScetweenHandler;

impl TagHandler for ScetweenHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let mut params = HashMap::new();
        for (key, value) in &ctx.instruction.params {
            let resolved = ctx.evaluator().resolve_param(value)?;
            params.insert(key.clone(), resolved.as_string());
        }
        Ok(TagResult::Emit(Event::TextAnimation(params)))
    }
}

/// [scein] 场景进入
pub struct SceinHandler;

impl TagHandler for SceinHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Emit(Event::SceneIn))
    }
}

/// [sceout] 场景退出
pub struct SceoutHandler;

impl TagHandler for SceoutHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Emit(Event::SceneOut))
    }
}

/// [automode] 自动模式设置
pub struct AutomodeHandler;

impl TagHandler for AutomodeHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let allow = ctx.instruction.get("allow")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1);
        let layer = ctx.instruction.get("layer").map(String::from);
        Ok(TagResult::Emit(Event::AutoModeConfig {
            allow: allow != 0,
            layer,
        }))
    }
}

/// [skip] 跳过设置
pub struct SkipHandler;

impl TagHandler for SkipHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let allow = ctx.instruction.get("allow")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1);
        let unread = ctx.instruction.get("unread")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);
        Ok(TagResult::Emit(Event::SkipConfig {
            allow: allow != 0,
            skip_unread: unread != 0,
        }))
    }
}

/// [backlog] 历史设置
pub struct BacklogHandler;

impl TagHandler for BacklogHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let allow = ctx.instruction.get("allow")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1);
        Ok(TagResult::Emit(Event::BacklogConfig { allow: allow != 0 }))
    }
}

/// [hide] 隐藏模式
pub struct HideHandler;

impl TagHandler for HideHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let allow = ctx.instruction.get("allow")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1);
        Ok(TagResult::Emit(Event::HideConfig { allow: allow != 0 }))
    }
}

/// [alreadyread] 已读判定设置
pub struct AlreadyreadHandler;

impl TagHandler for AlreadyreadHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let mode = ctx.instruction.get("mode")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);
        Ok(TagResult::Emit(Event::AlreadyReadConfig { mode }))
    }
}

/// [writebacklog] 历史写入设置
pub struct WritebacklogHandler;

impl TagHandler for WritebacklogHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let enable = ctx.instruction.get("enable")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1);
        Ok(TagResult::Emit(Event::WriteBacklogConfig { enable: enable != 0 }))
    }
}

/// [indent] 缩进设置
pub struct IndentHandler;

impl TagHandler for IndentHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let value = ctx.instruction.get("value").unwrap_or("").to_string();
        Ok(TagResult::Emit(Event::IndentConfig { value }))
    }
}

/// [prohibit] 禁则处理
pub struct ProhibitHandler;

impl TagHandler for ProhibitHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let value = ctx.instruction.get("value").unwrap_or("").to_string();
        Ok(TagResult::Emit(Event::ProhibitConfig { value }))
    }
}

/// [wordparts] 单词部分字符
pub struct WordpartsHandler;

impl TagHandler for WordpartsHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let value = ctx.instruction.get("value").unwrap_or("").to_string();
        Ok(TagResult::Emit(Event::WordpartsConfig { value }))
    }
}
