//! Lua Engine API
//!
//! 定义注入到 Lua 环境的 engine 对象（`e`）。
//! 每个通过 calllua 调用的 Lua 函数第一个参数都是 engine 对象。
//!
//! 大部分方法的实际行为需要由宿主应用（游戏引擎）通过回调提供。

use mlua::{Lua, Result as LuaResult, UserData, UserDataMethods, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 引擎事件回调 trait
///
/// 宿主应用实现此 trait 来响应 engine 对象的方法调用。
pub trait EngineCallbacks: Send + Sync {
    /// 输出调试日志
    fn debug(&self, level: i32, data: &str, raw: bool);

    /// 执行标签（返回标签名和参数）
    fn enqueue_tag(&self, tag: String, params: HashMap<String, String>);

    /// 设置事件处理器
    fn set_event_handler(&self, handlers: HashMap<String, String>);

    /// 获取脚本状态（0=执行中, 1=等待点击, 3=停止, 等）
    fn get_script_status(&self) -> u8;

    /// 检查按键是否按下
    fn is_key_down(&self, key_id: u32) -> bool;

    /// 检查按键是否刚按下
    fn is_key_down_edge(&self, key_id: u32) -> bool;

    /// 检查按键是否刚松开
    fn is_key_up_edge(&self, key_id: u32) -> bool;

    /// 检查确认键
    fn is_decide(&self) -> bool;

    /// 获取鼠标位置
    fn get_mouse_point(&self) -> (i32, i32);

    /// 获取触摸点数量
    fn get_touch_count(&self) -> u32;

    /// 获取触摸点位置
    fn get_touch_point(&self, index: u32) -> (i32, i32);

    /// 检查文件是否存在
    fn is_file_exists(&self, path: &str) -> bool;

    /// 文件操作
    fn file_operation(&self, command: &str, params: HashMap<String, String>);

    /// 加载 Lua 文件
    fn include(&self, path: &str);

    /// 覆盖按键
    fn override_key(&self, from: u32, to: u32);

    /// 设置滑动灵敏度
    fn set_flick_sensitivity(&self, sensitivity: f64);

    /// 获取脚本块信息
    fn get_script_block(&self) -> HashMap<String, String>;

    /// 获取脚本栈信息
    fn get_script_stack(&self) -> Vec<HashMap<String, String>>;

    /// 获取脚本等待原因
    fn get_script_wait_reason(&self) -> u8;
}

/// 默认的引擎回调实现（所有方法为空操作）
pub struct DefaultEngineCallbacks;

impl EngineCallbacks for DefaultEngineCallbacks {
    fn debug(&self, _level: i32, _data: &str, _raw: bool) {}
    fn enqueue_tag(&self, _tag: String, _params: HashMap<String, String>) {}
    fn set_event_handler(&self, _handlers: HashMap<String, String>) {}
    fn get_script_status(&self) -> u8 { 0 }
    fn is_key_down(&self, _key_id: u32) -> bool { false }
    fn is_key_down_edge(&self, _key_id: u32) -> bool { false }
    fn is_key_up_edge(&self, _key_id: u32) -> bool { false }
    fn is_decide(&self) -> bool { false }
    fn get_mouse_point(&self) -> (i32, i32) { (0, 0) }
    fn get_touch_count(&self) -> u32 { 0 }
    fn get_touch_point(&self, _index: u32) -> (i32, i32) { (0, 0) }
    fn is_file_exists(&self, _path: &str) -> bool { false }
    fn file_operation(&self, _command: &str, _params: HashMap<String, String>) {}
    fn include(&self, _path: &str) {}
    fn override_key(&self, _from: u32, _to: u32) {}
    fn set_flick_sensitivity(&self, _sensitivity: f64) {}
    fn get_script_block(&self) -> HashMap<String, String> { HashMap::new() }
    fn get_script_stack(&self) -> Vec<HashMap<String, String>> { vec![] }
    fn get_script_wait_reason(&self) -> u8 { 0 }
}

/// 共享的引擎上下文
pub struct EngineContext {
    pub callbacks: Box<dyn EngineCallbacks + Send + Sync>,
    /// 待执行的标签队列
    pub tag_queue: Vec<(String, HashMap<String, String>)>,
    /// 待设置的事件处理器
    pub event_handlers: HashMap<String, String>,
}

impl EngineContext {
    pub fn new(callbacks: Box<dyn EngineCallbacks + Send + Sync>) -> Self {
        Self {
            callbacks,
            tag_queue: Vec::new(),
            event_handlers: HashMap::new(),
        }
    }
}

/// Engine API 对象（注入到 Lua 的第一个参数）
pub struct EngineApi {
    ctx: Arc<Mutex<EngineContext>>,
}

impl EngineApi {
    pub fn new(ctx: Arc<Mutex<EngineContext>>) -> Self {
        Self { ctx }
    }
}

impl UserData for EngineApi {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // e:debug{level=0, data="foo", raw=false} 或 e:debug("foo")
        methods.add_method_mut("debug", |_lua, this, args: mlua::MultiValue| {
            let ctx = this.ctx.lock().unwrap();
            if args.is_empty() {
                return Ok(());
            }
            let first = args.iter().next().unwrap();
            match first {
                Value::String(s) => {
                    let data = s.to_str().map(|b| b.to_string()).unwrap_or_default();
                    ctx.callbacks.debug(0, &data, false);
                }
                Value::Table(t) => {
                    let level: i32 = t.get("level").unwrap_or(0);
                    let data: String = t.get("data").unwrap_or_default();
                    let raw: bool = t.get("raw").unwrap_or(false);
                    ctx.callbacks.debug(level, &data, raw);
                }
                _ => {}
            }
            Ok(())
        });

        // e:tag{"lyc", id="0", file="bg"}
        methods.add_method_mut("tag", |_lua, this, args: mlua::MultiValue| {
            if let Some(Value::Table(t)) = args.into_iter().next() {
                let tag_name: String = t.get(1).unwrap_or_default();
                let mut params = HashMap::new();
                for pair in t.pairs::<String, String>() {
                    if let Ok((k, v)) = pair {
                        params.insert(k, v);
                    }
                }
                // 移除数字键 1 (tag name)
                params.remove("1");
                let mut ctx = this.ctx.lock().unwrap();
                ctx.callbacks.enqueue_tag(tag_name.clone(), params.clone());
                ctx.tag_queue.push((tag_name, params));
            }
            Ok(())
        });

        // e:enqueueTag{"tagname", param1="val1"}
        methods.add_method_mut("enqueueTag", |_lua, this, args: mlua::MultiValue| {
            if let Some(Value::Table(t)) = args.into_iter().next() {
                let tag_name: String = t.get(1).unwrap_or_default();
                let mut params = HashMap::new();
                for pair in t.pairs::<String, String>() {
                    if let Ok((k, v)) = pair {
                        params.insert(k, v);
                    }
                }
                params.remove("1");
                let mut ctx = this.ctx.lock().unwrap();
                ctx.callbacks.enqueue_tag(tag_name.clone(), params.clone());
                ctx.tag_queue.push((tag_name, params));
            }
            Ok(())
        });

        // e:setEventHandler{onEnterFrame="func", ...}
        methods.add_method_mut("setEventHandler", |_lua, this, args: mlua::MultiValue| {
            if let Some(Value::Table(t)) = args.into_iter().next() {
                let mut handlers = HashMap::new();
                for pair in t.pairs::<String, String>() {
                    if let Ok((k, v)) = pair {
                        handlers.insert(k.clone(), v.clone());
                    }
                }
                let mut ctx = this.ctx.lock().unwrap();
                ctx.callbacks.set_event_handler(handlers.clone());
                ctx.event_handlers.extend(handlers);
            }
            Ok(())
        });

        // e:getScriptStatus()
        methods.add_method("getScriptStatus", |_lua, this, _: ()| {
            let ctx = this.ctx.lock().unwrap();
            Ok(ctx.callbacks.get_script_status())
        });

        // e:random()
        methods.add_method("random", |_lua, _this, _: ()| {
            Ok(rand_f64())
        });

        // e:now()
        methods.add_method("now", |_lua, _this, _: ()| {
            Ok(now_millis())
        });

        // e:include("path")
        methods.add_method("include", |_lua, this, path: String| {
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.include(&path);
            Ok(())
        });

        // e:isPush(key_id)
        methods.add_method("isPush", |_lua, this, key_id: u32| {
            let ctx = this.ctx.lock().unwrap();
            Ok(ctx.callbacks.is_key_down_edge(key_id))
        });

        // e:isDown(key_id)
        methods.add_method("isDown", |_lua, this, key_id: u32| {
            let ctx = this.ctx.lock().unwrap();
            Ok(ctx.callbacks.is_key_down(key_id))
        });

        // e:isDownEdge(key_id)
        methods.add_method("isDownEdge", |_lua, this, key_id: u32| {
            let ctx = this.ctx.lock().unwrap();
            Ok(ctx.callbacks.is_key_down_edge(key_id))
        });

        // e:isUpEdge(key_id)
        methods.add_method("isUpEdge", |_lua, this, key_id: u32| {
            let ctx = this.ctx.lock().unwrap();
            Ok(ctx.callbacks.is_key_up_edge(key_id))
        });

        // e:isDecide()
        methods.add_method("isDecide", |_lua, this, _: ()| {
            let ctx = this.ctx.lock().unwrap();
            Ok(ctx.callbacks.is_decide())
        });

        // e:getMousePoint()
        methods.add_method("getMousePoint", |_lua, this, _: ()| {
            let ctx = this.ctx.lock().unwrap();
            let (x, y) = ctx.callbacks.get_mouse_point();
            Ok((x, y))
        });

        // e:getTouchCount()
        methods.add_method("getTouchCount", |_lua, this, _: ()| {
            let ctx = this.ctx.lock().unwrap();
            Ok(ctx.callbacks.get_touch_count())
        });

        // e:getTouchPoint(index)
        methods.add_method("getTouchPoint", |_lua, this, index: u32| {
            let ctx = this.ctx.lock().unwrap();
            let (x, y) = ctx.callbacks.get_touch_point(index);
            Ok((x, y))
        });

        // e:file{command="copy", src="...", dst="..."}
        methods.add_method("file", |_lua, this, args: mlua::MultiValue| {
            if let Some(Value::Table(t)) = args.into_iter().next() {
                let command: String = t.get("command").unwrap_or_default();
                let mut params = HashMap::new();
                for pair in t.pairs::<String, String>() {
                    if let Ok((k, v)) = pair {
                        params.insert(k, v);
                    }
                }
                let ctx = this.ctx.lock().unwrap();
                ctx.callbacks.file_operation(&command, params);
            }
            Ok(())
        });

        // e:isFileExists("path")
        methods.add_method("isFileExists", |_lua, this, path: String| {
            let ctx = this.ctx.lock().unwrap();
            Ok(ctx.callbacks.is_file_exists(&path))
        });

        // e:overrideKey(from, to)
        methods.add_method("overrideKey", |_lua, this, (from, to): (u32, u32)| {
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.override_key(from, to);
            Ok(())
        });

        // e:setFlickSensitivity(sensitivity)
        methods.add_method("setFlickSensitivity", |_lua, this, sensitivity: f64| {
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.set_flick_sensitivity(sensitivity);
            Ok(())
        });

        // e:getScriptBlock()
        methods.add_method("getScriptBlock", |_lua, this, _: ()| {
            let ctx = this.ctx.lock().unwrap();
            Ok(ctx.callbacks.get_script_block())
        });

        // e:getScriptStack()
        methods.add_method("getScriptStack", |_lua, this, _: ()| {
            let ctx = this.ctx.lock().unwrap();
            Ok(ctx.callbacks.get_script_stack())
        });

        // e:getScriptWaitReason()
        methods.add_method("getScriptWaitReason", |_lua, this, _: ()| {
            let ctx = this.ctx.lock().unwrap();
            Ok(ctx.callbacks.get_script_wait_reason())
        });
    }
}

/// 当前时间（毫秒）
fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 简单随机数
fn rand_f64() -> f64 {
    let seed = now_millis();
    let x = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (x as f64) / (u64::MAX as f64)
}

/// 初始化 Lua 环境，注入 engine 对象
pub fn init_lua_engine_api(lua: &Lua, ctx: Arc<Mutex<EngineContext>>) -> LuaResult<()> {
    let engine = EngineApi::new(ctx);
    lua.globals().set("__engine", engine)?;
    Ok(())
}
