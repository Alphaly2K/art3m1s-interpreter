//! 事件处理器标签
//!
//! 实现 seton*/delon* 系列事件处理器标签。

use super::{ExecutionContext, TagHandler, TagResult};
use crate::error::Result;
use crate::event::Event;

/// 通用的事件处理器设置处理器
macro_rules! event_handler_struct {
    ($name:ident, $event_name:expr) => {
        pub struct $name;

        impl TagHandler for $name {
            fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
                let file = ctx.instruction.get("file").map(String::from);
                let label = ctx.instruction.get("label").map(String::from);
                let call = ctx
                    .instruction
                    .get("call")
                    .and_then(|v| v.parse::<i32>().ok())
                    .unwrap_or(0)
                    != 0;
                let handler = ctx.instruction.get("handler").map(String::from);

                // 已知字段以外的参数（key、adv、ui、btn 等）透传给宿主，
                // 宿主在事件触发时作为 Lua 回调的 param 表传回。
                let known = ["file", "label", "call", "handler"];
                let mut extra_params = std::collections::HashMap::new();
                for (k, v) in &ctx.instruction.params {
                    if !known.contains(&k.as_str()) {
                        extra_params.insert(k.clone(), v.clone());
                    }
                }

                Ok(TagResult::Emit(Event::SetEventHandler {
                    event_name: $event_name.to_string(),
                    file,
                    label,
                    call,
                    handler,
                    extra_params,
                }))
            }
        }
    };
}

/// 通用的事件处理器解除处理器
macro_rules! event_handler_del_struct {
    ($name:ident, $event_name:expr) => {
        pub struct $name;

        impl TagHandler for $name {
            fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
                // 带 key 时只解除该键，不带 key 时解除整个事件类型
                let key = ctx.instruction.get("key").map(String::from);
                Ok(TagResult::Emit(Event::DelEventHandler {
                    event_name: $event_name.to_string(),
                    key,
                }))
            }
        }
    };
}

// 事件处理器
event_handler_struct!(SetOnPushHandler, "push");
event_handler_struct!(SetOnAutomodeInHandler, "automodein");
event_handler_struct!(SetOnAutomodeOutHandler, "automodeout");
event_handler_struct!(SetOnBacklogInHandler, "backlogin");
event_handler_struct!(SetOnBacklogOutHandler, "backlogout");
event_handler_struct!(SetOnCommandSkipInHandler, "commandskipin");
event_handler_struct!(SetOnCommandSkipOutHandler, "commandskipout");
event_handler_struct!(SetOnControlSkipInHandler, "controlskipin");
event_handler_struct!(SetOnControlSkipOutHandler, "controlskipout");
event_handler_struct!(SetOnDirchgHandler, "dirchg");
event_handler_struct!(SetOnHideInHandler, "hidein");
event_handler_struct!(SetOnHideOutHandler, "hideout");
event_handler_struct!(SetOnWindowButtonHandler, "windowbutton");

// 事件处理器解除
event_handler_del_struct!(DelOnPushHandler, "push");
event_handler_del_struct!(DelOnAutomodeInHandler, "automodein");
event_handler_del_struct!(DelOnAutomodeOutHandler, "automodeout");
event_handler_del_struct!(DelOnBacklogInHandler, "backlogin");
event_handler_del_struct!(DelOnBacklogOutHandler, "backlogout");
event_handler_del_struct!(DelOnCommandSkipInHandler, "commandskipin");
event_handler_del_struct!(DelOnCommandSkipOutHandler, "commandskipout");
event_handler_del_struct!(DelOnControlSkipInHandler, "controlskipin");
event_handler_del_struct!(DelOnControlSkipOutHandler, "controlskipout");
event_handler_del_struct!(DelOnDirchgHandler, "dirchg");
event_handler_del_struct!(DelOnHideInHandler, "hidein");
event_handler_del_struct!(DelOnHideOutHandler, "hideout");
event_handler_del_struct!(DelOnWindowButtonHandler, "windowbutton");
