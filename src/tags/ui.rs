//! UI 相关标签处理器
//!
//! 实现 uitrans, lyc, lyc2, lydel, lyprop, loading, saving 等标签。

use super::{ExecutionContext, TagHandler, TagResult};
use crate::error::Result;
use crate::event::{
    Event, LayerEvent, LoadMaskAction, SystemUiAction, TransitionEvent,
};
use std::collections::HashMap;

/// [uitrans] UI 转场标签
pub struct UiTransHandler;

impl TagHandler for UiTransHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let time = ctx
            .instruction
            .get("time")
            .or_else(|| ctx.instruction.get("0"))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(500);

        let fade = ctx.instruction.get("fade").map(String::from);

        Ok(TagResult::Wait(Event::UiTransition(TransitionEvent {
            time,
            fade,
        })))
    }
}

/// [loading] 加载状态标签
pub struct LoadingHandler;

impl TagHandler for LoadingHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let active = ctx.instruction.get("0") == Some("on");

        Ok(TagResult::Emit(Event::LoadingState { active }))
    }
}

/// [saving] 存档状态标签
pub struct SavingHandler;

impl TagHandler for SavingHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let active = ctx.instruction.get("0") == Some("on");

        Ok(TagResult::Emit(Event::SavingState { active }))
    }
}

/// [sysshow] 系统 UI 显示标签
pub struct SysShowHandler;

impl TagHandler for SysShowHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Emit(Event::SystemUi {
            action: SystemUiAction::Show,
        }))
    }
}

/// [syshide] 系统 UI 隐藏标签
pub struct SysHideHandler;

impl TagHandler for SysHideHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let skip = ctx.instruction.get("skip").map(String::from);

        Ok(TagResult::Emit(Event::SystemUi {
            action: SystemUiAction::Hide { skip },
        }))
    }
}

/// [lyc] 创建图层标签
pub struct LycHandler;

impl TagHandler for LycHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.resolve_param_str("id")?;
        let file = ctx.resolve_param("file")?.as_string();

        Ok(TagResult::Emit(Event::Layer(LayerEvent::Create { id, file })))
    }
}

/// [lyc2] 创建图层标签（变体）
pub struct Lyc2Handler;

impl TagHandler for Lyc2Handler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.resolve_param_str("id")?;
        let file = ctx.resolve_param("file")?.as_string();
        let alpha = ctx
            .instruction
            .get("alpha")
            .and_then(|v| v.parse::<u8>().ok());

        Ok(TagResult::Emit(Event::Layer(LayerEvent::Create2 {
            id,
            file,
            alpha,
        })))
    }
}

/// [lydel] 删除图层标签
pub struct LydelHandler;

impl TagHandler for LydelHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.resolve_param_str("id")?;

        Ok(TagResult::Emit(Event::Layer(LayerEvent::Delete { id })))
    }
}

/// [lyprop] 图层属性标签
pub struct LypropHandler;

impl TagHandler for LypropHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.resolve_param_str("id")?;

        // 收集所有属性
        let mut properties = HashMap::new();
        for (key, value) in &ctx.instruction.params {
            if key != "id" {
                // 解析表达式
                let resolved = ctx.evaluator().resolve_param(value)?;
                properties.insert(key.clone(), resolved.as_string());
            }
        }

        // 如果是图层集（包含点），展开到所有子图层
        // 这里我们发出事件，由上层处理图层集逻辑
        Ok(TagResult::Emit(Event::Layer(LayerEvent::SetProperties {
            id,
            properties,
        })))
    }
}

/// [loadmask] 加载遮罩标签
pub struct LoadMaskHandler;

impl TagHandler for LoadMaskHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let action = match ctx.instruction.get("0") {
            Some("del") => LoadMaskAction::Delete,
            _ => LoadMaskAction::Show,
        };

        let time = ctx
            .instruction
            .get("time")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        Ok(TagResult::Emit(Event::LoadMask { action, time }))
    }
}

/// [alldelete] 全部删除标签
pub struct AllDeleteHandler;

impl TagHandler for AllDeleteHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let time = ctx
            .instruction
            .get("time")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        Ok(TagResult::Emit(Event::AllDelete { time }))
    }
}
