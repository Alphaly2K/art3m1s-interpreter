//! Lua 调用标签处理器
//!
//! 实现 calllua 标签和 [lua]/[/lua] 块。
//! 每个 Lua 函数都会接收一个 engine 对象作为第一个参数。

use super::{ExecutionContext, TagHandler, TagResult};
use crate::error::Result;

/// [calllua] 调用 Lua 函数
pub struct CallLuaHandler;

impl TagHandler for CallLuaHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let function_name = ctx.instruction.get("function").unwrap_or("");

        if function_name.is_empty() {
            return Err(crate::error::Error::RuntimeError {
                line: ctx.current_line,
                message: "calllua 缺少 function 参数".to_string(),
            });
        }

        // 收集额外参数（除 function 外的所有参数都传递给 Lua 函数）
        let mut extra_params = std::collections::HashMap::new();
        for (key, value) in &ctx.instruction.params {
            if key != "function" {
                extra_params.insert(key.clone(), value.clone());
            }
        }

        // 调用 Lua 函数，传入 engine 对象和额外参数
        call_lua_function(
            ctx.lua,
            ctx.variables,
            function_name,
            &extra_params,
        )?;

        Ok(TagResult::Continue)
    }
}

/// 调用 Lua 函数，将 engine 对象作为第一个参数传入
pub fn call_lua_function(
    lua: &mlua::Lua,
    _variables: &mut crate::variable::VariableStore,
    function_name: &str,
    _extra_params: &std::collections::HashMap<String, String>,
) -> Result<()> {
    // 处理嵌套函数名（如 "sv.save"）
    let parts: Vec<&str> = function_name.split('.').collect();

    // 尝试获取函数
    let func = if parts.len() > 1 {
        // 嵌套路径，如 sv.save
        let mut current: mlua::Value = lua.globals().get(parts[0])?;
        for &part in &parts[1..] {
            match current {
                mlua::Value::Table(t) => {
                    current = t.get(part)?;
                }
                _ => {
                    // 函数不存在，跳过
                    return Ok(());
                }
            }
        }
        match current {
            mlua::Value::Function(f) => f,
            _ => return Ok(()),
        }
    } else {
        // 全局函数
        let globals = lua.globals();
        match globals.get::<mlua::Function>(function_name) {
            Ok(f) => f,
            Err(_) => return Ok(()), // 函数不存在，跳过
        }
    };

    // 获取 engine 对象
    let engine: mlua::Value = lua.globals().get("__engine")?;

    // 调用函数，传入 engine 对象
    match engine {
        mlua::Value::UserData(ud) => {
            func.call::<()>((ud,))?;
        }
        _ => {
            // engine 对象不存在，调用无参数
            func.call::<()>(())?;
        }
    }

    Ok(())
}
