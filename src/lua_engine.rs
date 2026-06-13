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

    // ----------------------------------------------------------------
    // 以下为 boot 流程所需、暂以默认实现（no-op / 合理默认值）提供的回调。
    // 接入真实渲染/资源后端时，宿主可覆盖这些方法。
    // ----------------------------------------------------------------

    /// 设置脚本状态（如 4=停止）。
    fn set_script_status(&self, _status: u8) {}

    /// 注册标签过滤表（脚本侧保留该表引用，宿主据此过滤标签）。
    fn set_tag_filter(&self) {}

    /// 注册事件过滤表。
    fn set_event_filter(&self) {}

    /// 注册魔法路径别名（name -> path）。
    fn set_magic_path(&self, _name: &str, _path: &str) {}

    /// 设置多点触控模式。
    fn set_use_multi_touch(&self, _mode: i64) {}

    /// 设置是否启用触摸长按。
    fn set_use_touch_hold(&self, _enabled: bool) {}

    /// 调试跳转到指定脚本索引。
    fn debug_skip(&self, _index: i64) {}

    /// 文本编码转换。默认原样返回（项目本身为 UTF-8）。
    fn convert_encoding(&self, _from: &str, _to: &str, source: &str) -> String {
        source.to_string()
    }

    /// 执行外部 shell 命令（如打开 URL/文件）。
    fn call_shell_execute(&self, _file: &str, _params: HashMap<String, String>) {}

    /// 恢复字体缓存。
    fn restore_font_cache(&self, _path: &str) {}

    /// 读取 PNG 文件的注释块（如立绘坐标）。默认无注释返回 None。
    fn load_png_comments(&self, _path: &str) -> Option<HashMap<String, String>> {
        None
    }

    /// 异步绑定（预加载）surface。
    fn bind_surface_async(&self, _key: &str) {}

    /// 解绑（释放）surface。
    fn unbind_surface(&self, _key: &str) {}

    /// 清空 surface 加载队列。
    fn clear_surface_load_queue(&self) {}

    /// 是否仍有 surface 在异步加载中。默认 false（无后端即视为加载完成）。
    fn is_loading_surface(&self) -> bool {
        false
    }
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
    /// 项目文件读取器，供 `e:include` 读取 Lua/数据文件。
    ///
    /// `e:include(path)` 的语义是“读取文件并在当前 Lua VM 中执行”，因此 include
    /// 必须能读到项目文件。回调拿不到 Lua VM，故这里直接持有读取器，由 `e:include`
    /// 闭包用 mlua 传入的 `lua` 句柄重入执行。宿主通过
    /// [`Interpreter::set_file_loader`](crate::Interpreter::set_file_loader) 注入。
    pub file_reader: Option<FileReader>,
    /// 共享的变量存储，供 `e:var(name)` 读取解释器写入的变量。
    ///
    /// 与 [`Interpreter`](crate::Interpreter) 持有同一个 `Arc<Mutex<_>>`。
    pub variables: Option<Arc<Mutex<crate::variable::VariableStore>>>,
}

/// 文件读取器：给定虚拟路径返回原始字节。与
/// [`ScriptFileLoader`](crate::event::ScriptFileLoader) 同型，但用 `Arc` 以便在
/// 解释器与 [`EngineContext`] 之间共享同一个加载器。
pub type FileReader = Arc<dyn Fn(&str) -> crate::error::Result<Vec<u8>> + Send + Sync>;

impl EngineContext {
    pub fn new(callbacks: Box<dyn EngineCallbacks + Send + Sync>) -> Self {
        Self {
            callbacks,
            tag_queue: Vec::new(),
            event_handlers: HashMap::new(),
            file_reader: None,
            variables: None,
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
                // 用 Value 迭代，再手动转 String。mlua 的 `pairs::<String,String>()`
                // 一旦遇到非字符串值（如 boolean）就会终止迭代，后面的键全部丢失。
                // Lua 脚本传参经常带布尔字段（se=false 等），必须跳过而非中断。
                for pair in t.pairs::<mlua::Value, mlua::Value>() {
                    if let Ok((k, v)) = pair {
                        let key_str = match k {
                            Value::String(s) => s.to_str().ok().map(|s| s.to_string()),
                            Value::Integer(i) => Some(i.to_string()),
                            _ => None,
                        };
                        let val_str = match v {
                            Value::String(s) => s.to_str().ok().map(|s| s.to_string()),
                            Value::Integer(i) => Some(i.to_string()),
                            Value::Number(n) => Some(n.to_string()),
                            Value::Boolean(b) => Some(b.to_string()),
                            _ => None,
                        };
                        if let (Some(ks), Some(vs)) = (key_str, val_str) {
                            params.insert(ks, vs);
                        }
                    }
                }
                // 移除数字键 1 (tag name)
                params.remove("1");
                let mut ctx = this.ctx.lock().unwrap();
                ctx.callbacks.enqueue_tag(tag_name.clone(), params.clone());

                // var 标签不产出事件，且脚本常在同一 Lua 函数内紧接着用 e:var 读回
                // 刚设的值。若只入队、等 calllua 返回后才 flush，就会读到 nil。故 var
                // 在此同步落值到共享存储，并且不入队（入队 flush 只会重复落值，对
                // random 等有副作用的变体反而有害）。其余标签照常入队走事件管线。
                if tag_name == "var" {
                    if let Some(vars) = &ctx.variables {
                        let mut store = vars.lock().unwrap();
                        if let Err(e) = crate::tags::var_handler::apply_var_tag(&params, &mut store) {
                            return Err(mlua::Error::external(format!("var 标签执行失败: {e}")));
                        }
                    }
                } else {
                    ctx.tag_queue.push((tag_name, params));
                }
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
        //
        // 读取项目文件并在**当前** Lua VM 中执行——这是 include 的真正语义
        // （注册函数、建表等副作用必须落在同一个 VM 里）。注意：被 include 的
        // Lua 代码会反过来调用 `e:` 方法（如 e:tag/e:var/e:include 嵌套），这些都会
        // 再次锁 ctx，所以必须先克隆出 file_reader 并释放锁，再读文件、再 exec，
        // 否则会自锁死。
        methods.add_method("include", |lua, this, path: String| {
            let reader = {
                let ctx = this.ctx.lock().unwrap();
                // 仍通知回调（宿主可借此记录/追踪 include），但实际加载在此处完成。
                ctx.callbacks.include(&path);
                ctx.file_reader.clone()
            };

            let Some(reader) = reader else {
                // 未注入读取器：保持旧的“仅通知回调”行为，不视为错误。
                return Ok(());
            };

            let bytes = reader(&path)
                .map_err(|e| mlua::Error::external(format!("include 读取 {path} 失败: {e}")))?;

            // lua.load 接受 &[u8]，文本源码与 luac 字节码均可。
            lua.load(&bytes[..])
                .set_name(path.as_str())
                .exec()
                .map_err(|e| mlua::Error::external(format!("include 执行 {path} 失败: {e}")))?;

            Ok(())
        });

        // e:var("name") -> 读取共享变量存储中的变量值。
        //
        // 按变量原始类型返回对应 Lua 值（整数/浮点/字符串/布尔），不存在或 Null 返回
        // nil。这样脚本里 `tn(e:var(..))`（tonumber）、数值比较与 `if e:var(..) then`
        // 三种用法都成立。变量由 var 标签（e:tag{"var",...}）写入。
        methods.add_method("var", |lua, this, name: String| {
            use crate::variable::Value as V;
            let ctx = this.ctx.lock().unwrap();
            let Some(vars) = &ctx.variables else {
                return Ok(mlua::Value::Nil);
            };
            let store = vars.lock().unwrap();
            match store.get(&name) {
                Some(V::Int(n)) => Ok(mlua::Value::Integer(*n)),
                Some(V::Float(f)) => Ok(mlua::Value::Number(*f)),
                Some(V::Bool(b)) => Ok(mlua::Value::Boolean(*b)),
                Some(V::String(s)) => Ok(mlua::Value::String(lua.create_string(s)?)),
                Some(V::Null) | None => Ok(mlua::Value::Nil),
            }
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

        // ------------------------------------------------------------
        // boot 流程所需方法（多数转发到 EngineCallbacks 默认实现）。
        // ------------------------------------------------------------

        // e:setScriptStatus(status)
        methods.add_method("setScriptStatus", |_lua, this, status: i64| {
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.set_script_status(status as u8);
            Ok(())
        });

        // e:setTagFilter(tags) — 参数为 Lua 表，脚本侧持有其引用；这里仅通知宿主。
        methods.add_method("setTagFilter", |_lua, this, _tags: mlua::Value| {
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.set_tag_filter();
            Ok(())
        });

        // e:setEventFilter(filter)
        methods.add_method("setEventFilter", |_lua, this, _filter: mlua::Value| {
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.set_event_filter();
            Ok(())
        });

        // e:setMagicPath{name, path} — 位置 1=name, 位置 2=path。
        methods.add_method("setMagicPath", |_lua, this, t: mlua::Table| {
            let name: String = t.get(1).unwrap_or_default();
            let path: String = t.get(2).unwrap_or_default();
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.set_magic_path(&name, &path);
            Ok(())
        });

        // e:setUseMultiTouch(mode)
        methods.add_method("setUseMultiTouch", |_lua, this, mode: i64| {
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.set_use_multi_touch(mode);
            Ok(())
        });

        // e:setUseTouchHold(enabled)
        methods.add_method("setUseTouchHold", |_lua, this, enabled: bool| {
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.set_use_touch_hold(enabled);
            Ok(())
        });

        // e:debugSkip{index=...}
        methods.add_method("debugSkip", |_lua, this, t: mlua::Table| {
            let index: i64 = t.get("index").unwrap_or(0);
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.debug_skip(index);
            Ok(())
        });

        // e:convertEncoding{from=, to=, source=} -> String
        methods.add_method("convertEncoding", |_lua, this, t: mlua::Table| {
            let from: String = t.get("from").unwrap_or_default();
            let to: String = t.get("to").unwrap_or_default();
            let source: String = t.get("source").unwrap_or_default();
            let ctx = this.ctx.lock().unwrap();
            Ok(ctx.callbacks.convert_encoding(&from, &to, &source))
        });

        // e:callShellExecute{file=..., ...}
        methods.add_method("callShellExecute", |_lua, this, t: mlua::Table| {
            let file: String = t.get("file").unwrap_or_default();
            let mut params = HashMap::new();
            for pair in t.pairs::<String, String>() {
                if let Ok((k, v)) = pair {
                    if k != "file" {
                        params.insert(k, v);
                    }
                }
            }
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.call_shell_execute(&file, params);
            Ok(())
        });

        // e:restoreFontCache(path)
        methods.add_method("restoreFontCache", |_lua, this, path: String| {
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.restore_font_cache(&path);
            Ok(())
        });

        // e:loadPngComments(path) -> { comment=... } | nil
        methods.add_method("loadPngComments", |lua, this, path: String| {
            let ctx = this.ctx.lock().unwrap();
            match ctx.callbacks.load_png_comments(&path) {
                Some(map) => {
                    let t = lua.create_table()?;
                    for (k, v) in map {
                        t.set(k, v)?;
                    }
                    Ok(mlua::Value::Table(t))
                }
                None => Ok(mlua::Value::Nil),
            }
        });

        // e:bindSurfaceAsync(key)
        methods.add_method("bindSurfaceAsync", |_lua, this, key: mlua::Value| {
            let key = lua_value_to_key(&key);
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.bind_surface_async(&key);
            Ok(())
        });

        // e:unbindSurface(key)
        methods.add_method("unbindSurface", |_lua, this, key: mlua::Value| {
            let key = lua_value_to_key(&key);
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.unbind_surface(&key);
            Ok(())
        });

        // e:clearSurfaceLoadQueue()
        methods.add_method("clearSurfaceLoadQueue", |_lua, this, _: ()| {
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.clear_surface_load_queue();
            Ok(())
        });

        // e:isLoadingSurface(_) -> bool
        methods.add_method("isLoadingSurface", |_lua, this, _arg: mlua::Value| {
            let ctx = this.ctx.lock().unwrap();
            Ok(ctx.callbacks.is_loading_surface())
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

/// 把 surface 键参数转成字符串。boot 里 bind/unbindSurface 传入的多为路径字符串，
/// 也可能是数值索引；其它类型暂转为空串（stub 阶段不区分具体 surface）。
fn lua_value_to_key(v: &mlua::Value) -> String {
    match v {
        mlua::Value::String(s) => s.to_str().map(|s| s.to_string()).unwrap_or_default(),
        mlua::Value::Integer(n) => n.to_string(),
        mlua::Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
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
