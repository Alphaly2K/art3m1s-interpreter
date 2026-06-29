//! 图层相关标签处理器
//!
//! 实现图层操作、缓动、事件、动画、视频、转场、截图等标签。

use crate::error::Result;
use crate::event::Event;
use crate::tags::{ExecutionContext, TagHandler, TagResult};
use std::collections::HashMap;

/// [lytween] 图层缓动处理器
pub struct LytweenHandler;

impl TagHandler for LytweenHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.instruction.get("id").unwrap_or("").to_string();
        let param = ctx.instruction.get("param").unwrap_or("").to_string();
        let from = ctx.instruction.get("from").map(|s| s.to_string());
        let to = ctx.instruction.get("to").map(|s| s.to_string());
        let ease = ctx.instruction.get("ease").map(|s| s.to_string());
        let time = ctx
            .instruction
            .get("time")
            .and_then(|s| s.parse::<u64>().ok());
        let delay = ctx
            .instruction
            .get("delay")
            .and_then(|s| s.parse::<u64>().ok());
        let loop_count = ctx
            .instruction
            .get("loop")
            .and_then(|s| s.parse::<i32>().ok());
        let yoyo = ctx
            .instruction
            .get("yoyo")
            .and_then(|s| s.parse::<i32>().ok());
        let loop_delay = ctx
            .instruction
            .get("loopdelay")
            .and_then(|s| s.parse::<u64>().ok());
        let sync = ctx
            .instruction
            .get("sync")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0)
            != 0;
        let delete = ctx
            .instruction
            .get("delete")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0)
            != 0;
        let handler_file = ctx.instruction.get("file").map(|s| s.to_string());
        let handler_label = ctx.instruction.get("label").map(|s| s.to_string());
        let handler_handler = ctx.instruction.get("handler").map(|s| s.to_string());

        Ok(TagResult::Emit(Event::LayerTween {
            id,
            param,
            from,
            to,
            ease,
            time,
            delay,
            loop_count,
            yoyo,
            loop_delay,
            sync,
            delete,
            handler_file,
            handler_label,
            handler_handler,
        }))
    }
}

/// [lytweendel] 删除图层缓动处理器
pub struct LytweendelHandler;

impl TagHandler for LytweendelHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.instruction.get("id").unwrap_or("").to_string();
        Ok(TagResult::Emit(Event::LayerTweenDelete { id }))
    }
}

/// [tweenset] 缓动序列开始处理器
pub struct TweensetHandler;

impl TagHandler for TweensetHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Emit(Event::TweenSetStart))
    }
}

/// [/tweenset] 缓动序列结束处理器
pub struct TweensetEndHandler;

impl TagHandler for TweensetEndHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Emit(Event::TweenSetEnd))
    }
}

/// [lyevent] 图层事件处理器
pub struct LyeventHandler;

impl TagHandler for LyeventHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.instruction.get("id").unwrap_or("").to_string();
        let event_type = ctx.instruction.get("type").unwrap_or("").to_string();
        let mode = ctx.instruction.get("mode").unwrap_or("").to_string();
        let file = ctx.instruction.get("file").map(|s| s.to_string());
        let label = ctx.instruction.get("label").map(|s| s.to_string());
        let call = ctx
            .instruction
            .get("call")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0)
            != 0;
        let handler = ctx.instruction.get("handler").map(|s| s.to_string());
        let penetration = ctx
            .instruction
            .get("penetration")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0)
            != 0;

        // 把已知字段以外的参数收集为 extra_params（function、name、key、se 等）。
        // 这些会在事件触发时由宿主原样塞进 handler 标签（如 calllua）的参数表，
        // 引擎不解释其语义。
        let known = [
            "id",
            "type",
            "mode",
            "file",
            "label",
            "call",
            "handler",
            "penetration",
        ];
        let mut base_extra_params = std::collections::HashMap::new();
        for (k, v) in ctx.instruction.params.iter() {
            if !known.contains(&k.as_str()) {
                base_extra_params.insert(k.clone(), v.clone());
            }
        }

        Ok(TagResult::Emit(Event::LayerEventHandler {
            id,
            event_type,
            mode,
            file,
            label,
            call,
            handler,
            penetration,
            extra_params: base_extra_params,
        }))
    }
}

/// [lyrename] 图层重命名处理器
pub struct LyrenameHandler;

impl TagHandler for LyrenameHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.instruction.get("id").unwrap_or("").to_string();
        let to = ctx.instruction.get("to").unwrap_or("").to_string();
        Ok(TagResult::Emit(Event::LayerRename { id, to }))
    }
}

/// [lyedit] 图层编辑处理器
pub struct LyeditHandler;

impl TagHandler for LyeditHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.instruction.get("id").unwrap_or("").to_string();
        let mode = ctx.instruction.get("mode").unwrap_or("").to_string();
        let color = ctx.instruction.get("color").map(|s| s.to_string());
        let file = ctx.instruction.get("file").map(|s| s.to_string());
        let left = ctx
            .instruction
            .get("left")
            .and_then(|s| s.parse::<i32>().ok());
        let top = ctx
            .instruction
            .get("top")
            .and_then(|s| s.parse::<i32>().ok());

        Ok(TagResult::Emit(Event::LayerEdit {
            id,
            mode,
            color,
            file,
            left,
            top,
        }))
    }
}

/// [lydrag] 图层拖动处理器
pub struct LydragHandler;

impl TagHandler for LydragHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.instruction.get("id").unwrap_or("").to_string();
        Ok(TagResult::Emit(Event::LayerDrag { id }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::Instruction;
    use crate::tags::ExecutionContext;
    use crate::variable::VariableStore;

    #[test]
    fn lyevent_custom_click_over_out_params_are_only_forwarded() {
        let lua = mlua::Lua::new();
        let instruction = Instruction {
            tag: "lyevent".into(),
            params: HashMap::from([
                ("id".into(), "dock".into()),
                ("type".into(), "click".into()),
                ("handler".into(), "calllua".into()),
                ("function".into(), "btn_clickex".into()),
                ("click".into(), "btn_click".into()),
                ("over".into(), "mwarea_over".into()),
                ("out".into(), "mwarea_out".into()),
            ]),
            line: 1,
        };
        let mut variables = VariableStore::new();
        let get_script = |_name: &str| None;
        let mut ctx = ExecutionContext {
            variables: &mut variables,
            lua: &lua,
            current_script: "test",
            current_line: 0,
            instruction: &instruction,
            get_script: &get_script,
        };

        let TagResult::Emit(Event::LayerEventHandler {
            id,
            event_type,
            handler,
            extra_params,
            ..
        }) = LyeventHandler.execute(&mut ctx).unwrap()
        else {
            panic!("lyevent should produce a single documented event registration");
        };

        assert_eq!(id, "dock");
        assert_eq!(event_type, "click");
        assert_eq!(handler.as_deref(), Some("calllua"));
        assert_eq!(
            extra_params.get("function").map(String::as_str),
            Some("btn_clickex")
        );
        assert_eq!(
            extra_params.get("click").map(String::as_str),
            Some("btn_click")
        );
        assert_eq!(
            extra_params.get("over").map(String::as_str),
            Some("mwarea_over")
        );
        assert_eq!(
            extra_params.get("out").map(String::as_str),
            Some("mwarea_out")
        );
    }
}

/// [anime] 动画处理器
pub struct AnimeHandler;

impl TagHandler for AnimeHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.instruction.get("id").unwrap_or("").to_string();
        let mode = ctx.instruction.get("mode").unwrap_or("").to_string();
        let file = ctx.instruction.get("file").map(|s| s.to_string());
        let mask = ctx.instruction.get("mask").map(|s| s.to_string());
        let time = ctx
            .instruction
            .get("time")
            .and_then(|s| s.parse::<u64>().ok());
        let loop_count = ctx
            .instruction
            .get("loop")
            .and_then(|s| s.parse::<i32>().ok());

        let mut props = HashMap::new();
        for key in &[
            "left",
            "top",
            "alpha",
            "anchorx",
            "anchory",
            "xscale",
            "yscale",
            "rotate",
            "reversex",
            "reversey",
            "clip",
            "layermode",
            "negative",
            "grayscale",
            "colormultiply",
            "visible",
        ] {
            if let Some(val) = ctx.instruction.get(key) {
                props.insert(key.to_string(), val.to_string());
            }
        }

        Ok(TagResult::Emit(Event::Anime {
            id,
            mode,
            file,
            mask,
            time,
            loop_count,
            props,
        }))
    }
}

/// [video] 视频处理器
pub struct VideoHandler;

impl TagHandler for VideoHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.instruction.get("id").map(|s| s.to_string());
        let file = ctx.instruction.get("file").unwrap_or("").to_string();
        let skip = ctx
            .instruction
            .get("skip")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(1)
            != 0;
        let loop_play = ctx
            .instruction
            .get("loop")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0)
            != 0;

        Ok(TagResult::Emit(Event::VideoPlay {
            id,
            file,
            skip,
            loop_play,
        }))
    }
}

/// [setonvideofinish] 设置视频完成事件处理器
pub struct SetOnVideofinishHandler;

impl TagHandler for SetOnVideofinishHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let file = ctx.instruction.get("file").map(|s| s.to_string());
        let label = ctx.instruction.get("label").map(|s| s.to_string());
        let call = ctx
            .instruction
            .get("call")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0)
            != 0;
        let handler = ctx.instruction.get("handler").map(|s| s.to_string());

        Ok(TagResult::Emit(Event::VideoFinishHandler {
            file,
            label,
            call,
            handler,
        }))
    }
}

/// [delonvideofinish] 删除视频完成事件处理器
pub struct DelOnVideofinishHandler;

impl TagHandler for DelOnVideofinishHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Emit(Event::VideoFinishHandlerDel))
    }
}

/// [trans] 转场处理器
pub struct TransHandler;

impl TagHandler for TransHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let trans_type = ctx
            .instruction
            .get("type")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(1);
        let time = ctx
            .instruction
            .get("time")
            .and_then(|s| s.parse::<u64>().ok());
        let rule = ctx.instruction.get("rule").map(|s| s.to_string());
        let vague = ctx
            .instruction
            .get("vague")
            .and_then(|s| s.parse::<i32>().ok());
        let input = ctx
            .instruction
            .get("input")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(1);

        Ok(TagResult::Emit(Event::Trans {
            trans_type,
            time,
            rule,
            vague,
            input,
        }))
    }
}

/// [flip] 立即反映处理器
pub struct FlipHandler;

impl TagHandler for FlipHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Emit(Event::Flip))
    }
}

/// [takess] 截图处理器
pub struct TakessHandler;

impl TagHandler for TakessHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Emit(Event::TakeScreenshot))
    }
}

/// [savess] 保存截图处理器
pub struct SavessHandler;

impl TagHandler for SavessHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let file = ctx.instruction.get("file").unwrap_or("").to_string();
        let width = ctx
            .instruction
            .get("width")
            .and_then(|value| value.parse::<u32>().ok());
        let height = ctx
            .instruction
            .get("height")
            .and_then(|value| value.parse::<u32>().ok());
        Ok(TagResult::Emit(Event::SaveScreenshot {
            file,
            width,
            height,
        }))
    }
}

/// [rclick] 右键菜单处理器
pub struct RclickHandler;

impl TagHandler for RclickHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let allow = ctx
            .instruction
            .get("allow")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(1)
            != 0;
        let file = ctx.instruction.get("file").map(|s| s.to_string());

        Ok(TagResult::Emit(Event::RightClickConfig { allow, file }))
    }
}

/// [macrodel] 删除宏处理器
pub struct MacrodelHandler;

impl TagHandler for MacrodelHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let file = ctx.instruction.get("file").unwrap_or("").to_string();
        Ok(TagResult::Emit(Event::MacroDel { file }))
    }
}
