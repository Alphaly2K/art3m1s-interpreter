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

    // 查询图层信息，供 `var system="get_layer_info"` 这类同步查询使用。
    fn get_layer_info(&self, _id: &str) -> Option<HashMap<String, String>> {
        None
    }

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

    // ── Audio volume ──────────────────────────────────────────

    fn set_master_volume(&self, _volume: f32) {}
    fn set_bgm_volume(&self, _volume: f32) {}
    fn set_se_volume(&self, _volume: f32) {}
    fn set_voice_volume(&self, _volume: f32) {}

    fn call_shell_execute(&self, _file: &str, _params: HashMap<String, String>) {}

    /// 恢复字体缓存。
    fn restore_font_cache(&self, _path: &str) {}

    /// 读取 PNG 文件的注释块（如立绘坐标）。默认无注释返回 None。
    fn load_png_comments(&self, _path: &str) -> Option<HashMap<String, String>> {
        None
    }

    /// 同步绑定 surface（阻塞直到加载完成）。
    fn bind_surface(&self, _key: &str) {}

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
    fn get_script_status(&self) -> u8 {
        0
    }
    fn is_key_down(&self, _key_id: u32) -> bool {
        false
    }
    fn is_key_down_edge(&self, _key_id: u32) -> bool {
        false
    }
    fn is_key_up_edge(&self, _key_id: u32) -> bool {
        false
    }
    fn is_decide(&self) -> bool {
        false
    }
    fn get_mouse_point(&self) -> (i32, i32) {
        (0, 0)
    }
    fn get_touch_count(&self) -> u32 {
        0
    }
    fn get_touch_point(&self, _index: u32) -> (i32, i32) {
        (0, 0)
    }
    fn is_file_exists(&self, _path: &str) -> bool {
        false
    }
    fn file_operation(&self, _command: &str, _params: HashMap<String, String>) {}
    fn include(&self, _path: &str) {}
    fn override_key(&self, _from: u32, _to: u32) {}
    fn set_flick_sensitivity(&self, _sensitivity: f64) {}
    fn get_script_block(&self) -> HashMap<String, String> {
        HashMap::new()
    }
    fn get_script_stack(&self) -> Vec<HashMap<String, String>> {
        vec![]
    }
    fn get_script_wait_reason(&self) -> u8 {
        0
    }
}

/// 共享的引擎上下文
pub struct EngineContext {
    pub callbacks: Box<dyn EngineCallbacks + Send + Sync>,
    /// 待执行的标签队列
    pub tag_queue: Vec<(String, HashMap<String, String>)>,
    /// 待设置的事件处理器
    pub event_handlers: HashMap<String, String>,
    /// 事件过滤器函数（由 e:setEventFilter 设置）
    /// 当输入事件发生时，引擎调用此过滤器，脚本决定如何处理
    pub event_filter: Option<mlua::RegistryKey>,
    /// 项目文件读取器，供 `e:include` 读取 Lua/数据文件。
    ///
    /// `e:include(path)` 的语义是”读取文件并在当前 Lua VM 中执行”，因此 include
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
            event_filter: None,
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

    /// 分发事件通过事件过滤器。
    /// 返回 Some(0/1/2) 表示过滤器已处理，None 表示没有过滤器或过滤器返回 0（引擎处理）。
    /// - 0: 引擎正常处理
    /// - 1: 脚本已处理，引擎什么都不做
    /// - 2: 分发失败，引擎执行默认行为
    pub fn dispatch_event(
        &self,
        lua: &mlua::Lua,
        event_name: &str,
        params: &HashMap<String, String>,
    ) -> Option<i32> {
        let ctx = self.ctx.lock().unwrap();
        let filter_key = ctx.event_filter.as_ref()?;

        // 从 registry 获取过滤器函数
        let filter: mlua::Function = match lua.registry_value(filter_key) {
            Ok(f) => f,
            Err(_) => return None,
        };

        // 构造参数表
        let params_table = match lua.create_table() {
            Ok(t) => t,
            Err(_) => return None,
        };
        for (k, v) in params {
            let _ = params_table.set(k.as_str(), v.as_str());
        }

        // 调用过滤器: eventFilter(e, nm, p)
        // e 是 EngineApi 自身（self），但这里我们不能直接传递 self
        // 所以传递一个简化的事件对象
        let event_obj = match lua.create_table() {
            Ok(t) => t,
            Err(_) => return None,
        };

        match filter.call::<i32>((event_obj, event_name, params_table)) {
            Ok(result) => Some(result),
            Err(e) => {
                ctx.callbacks
                    .debug(0, &format!("eventFilter error: {}", e), false);
                None
            }
        }
    }
}

impl UserData for EngineApi {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // e:debug{level=0, data="foo", raw=false} 或 e:debug("foo")
        methods.add_method("debug", |_lua, this, args: mlua::MultiValue| {
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
        methods.add_method("tag", |_lua, this, args: mlua::MultiValue| {
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
                        let handled =
                            if params.get("system").map(String::as_str) == Some("get_layer_info") {
                                let name = params.get("name").map(String::as_str).unwrap_or("");
                                let id = params.get("id").map(String::as_str).unwrap_or("");
                                if let Some(info) = ctx.callbacks.get_layer_info(id) {
                                    for (key, value) in info {
                                        let value = value
                                            .parse::<f64>()
                                            .map(crate::variable::Value::Float)
                                            .unwrap_or(crate::variable::Value::String(value));
                                        store.set(&format!("{name}.{key}"), value);
                                    }
                                    true
                                } else {
                                    false
                                }
                            } else {
                                false
                            };

                        if !handled
                            && let Err(e) =
                                crate::tags::var_handler::apply_var_tag(&params, &mut store)
                        {
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
        methods.add_method("enqueueTag", |lua, this, args: mlua::MultiValue| {
            if let Some(Value::Table(t)) = args.into_iter().next() {
                let tag_name: String = t
                    .get::<Value>(1)
                    .ok()
                    .and_then(|v| match v {
                        Value::String(s) => s.to_str().ok().map(|s| s.to_string()),
                        Value::Integer(i) => Some(i.to_string()),
                        _ => None,
                    })
                    .unwrap_or_default();
                let mut params = HashMap::new();
                let mut real_param_table: Option<mlua::Table> = None;
                for pair in t.pairs::<Value, Value>() {
                    if let Ok((k, v)) = pair {
                        let key_str = match &k {
                            Value::String(s) => s.to_str().ok().map(|s| s.to_string()),
                            Value::Integer(i) => Some(i.to_string()),
                            _ => None,
                        };
                        if let Some(ks) = key_str {
                            match v {
                                Value::Table(vt) if ks == "params" => {
                                    real_param_table = Some(vt.clone());
                                }
                                Value::Table(_) => {} // 其他表值跳过
                                _ => {
                                    let val_str = match v {
                                        Value::String(s) => s.to_str().ok().map(|s| s.to_string()),
                                        Value::Integer(i) => Some(i.to_string()),
                                        Value::Number(n) => Some(n.to_string()),
                                        Value::Boolean(b) => Some(b.to_string()),
                                        _ => None,
                                    };
                                    if let Some(vs) = val_str {
                                        params.insert(ks, vs);
                                    }
                                }
                            }
                        }
                    }
                }
                params.remove("1");
                let mut ctx = this.ctx.lock().unwrap();
                ctx.callbacks.enqueue_tag(tag_name.clone(), params.clone());
                // calllua + 含表 params → 表数据无法序列化进 HashMap，同步执行，
                // 仿 e:tag 对 var 标签的特判。
                if tag_name == "calllua" {
                    if let Some(function_name) = params.get("function").cloned() {
                        drop(ctx);
                        if let Some(real_pt) = real_param_table {
                            let param_table =
                                lua.create_table().map_err(|e| mlua::Error::external(e))?;
                            for (k, v) in &params {
                                if k != "function" && k != "params" {
                                    let _ = param_table.set(k.as_str(), v.as_str());
                                }
                            }
                            for pair in real_pt.pairs::<Value, Value>() {
                                if let Ok((k, v)) = pair {
                                    let _ = param_table.set(k, v);
                                }
                            }
                            if let Err(e) = crate::tags::call_lua_function_with_table(
                                lua,
                                &function_name,
                                param_table,
                            ) {
                                return Err(mlua::Error::external(format!(
                                    "calllua 执行失败: {e}"
                                )));
                            }
                            return Ok(());
                        }
                        crate::tags::call_lua_function(lua, &function_name, &params)
                            .map_err(|e| mlua::Error::external(format!("calllua 执行失败: {e}")))?;
                        return Ok(());
                    }
                }
                ctx.tag_queue.push((tag_name, params));
            }
            Ok(())
        });

        // e:setEventHandler{onEnterFrame="func", ...}
        methods.add_method("setEventHandler", |_lua, this, args: mlua::MultiValue| {
            if let Some(Value::Table(t)) = args.into_iter().next() {
                let mut handlers = HashMap::new();
                for pair in t.pairs::<Value, Value>() {
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
                            handlers.insert(ks, vs);
                        }
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
        methods.add_method("random", |_lua, _this, _: ()| Ok(rand_i64()));

        // e:now()
        methods.add_method("now", |_lua, _this, _: ()| Ok(now_millis()));

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
        // 按变量原始类型返回对应 Lua 值（整数/浮点/字符串/布尔）。不存在或 Null 的
        // 变量返回字符串 "0"——这是 Artemis 的核心约定（var.txt 标注返回类型为
        // string，从不为 nil；脚本据此写 `if e:var(..) ~= "0"`、`== "0"`，例如
        // system/adv/fileio.lua 的 fload_pluto 靠它判断配置是否首次创建）。返回
        // nil 会让那些判断误判，导致 boot 在"系统数据已初始化"对话框处卡死。
        // 数值用 tn(e:var(..)) 解析时 tn("0")==0，比较与 `if .. then` 也都成立
        // （Lua 中 "0" 为真）。
        methods.add_method("var", |lua, this, name: String| {
            use crate::variable::Value as V;
            let ctx = this.ctx.lock().unwrap();
            let Some(vars) = &ctx.variables else {
                return Ok(mlua::Value::String(lua.create_string("0")?));
            };
            let store = vars.lock().unwrap();
            match store.get(&name) {
                Some(V::Int(n)) => Ok(mlua::Value::Integer(*n)),
                Some(V::Float(f)) => Ok(mlua::Value::Number(*f)),
                // Artemis 变量系统没有独立布尔类型：比较/逻辑表达式（如 `$0==0`）
                // 的结果在脚本里一律当整数 1/0 用，game 侧普遍写成
                // `tn(e:var(...))`（即 tonumber）。若把 Bool 作为 Lua boolean 返回，
                // tonumber(true) 得到 nil，cond() 会把成立的条件误判为 false
                // （典型：brandlogo 的 `cond="s.sp==0"` 被跳过）。故在此折叠成 1/0。
                Some(V::Bool(b)) => Ok(mlua::Value::Integer(if *b { 1 } else { 0 })),
                Some(V::String(s)) => Ok(mlua::Value::String(lua.create_string(s)?)),
                Some(V::Null) | None => Ok(mlua::Value::String(lua.create_string("0")?)),
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

        // e:getMousePoint() -> { x=, y= }
        // 脚本一律按 table 取用（`pos.x` / `pos.y`，见 button.lua slider_clickX、
        // adv.lua 等数十处）。Lua 里 `local pos = e:getMousePoint()` 只接收首个
        // 返回值，若返回 tuple 则 pos 是数字，`pos.x` 会触发 index a number 错误
        // （表现为 config 滑动条点击/拖动报 LuaError、滑条失灵）。
        methods.add_method("getMousePoint", |lua, this, _: ()| {
            let (x, y) = {
                let ctx = this.ctx.lock().unwrap();
                ctx.callbacks.get_mouse_point()
            };
            let t = lua.create_table()?;
            t.set("x", x)?;
            t.set("y", y)?;
            Ok(t)
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

        // e:overrideKey{ key=id, status=0 } 或 e:overrideKey(from, to)
        methods.add_method("overrideKey", |_lua, this, args: mlua::MultiValue| {
            let mut from: u32 = 0;
            let mut to: u32 = 0;
            if let Some(first) = args.iter().next() {
                match first {
                    mlua::Value::Table(t) => {
                        from = t.get("key").unwrap_or(0u32);
                        to = t.get("status").unwrap_or(0u32);
                    }
                    mlua::Value::Integer(n) => {
                        from = *n as u32;
                        if args.len() >= 2 {
                            if let Some(mlua::Value::Integer(n2)) = args.iter().nth(1) {
                                to = *n2 as u32;
                            }
                        }
                    }
                    _ => {}
                }
            }
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
        methods.add_method("setEventFilter", |lua, this, filter: mlua::Value| {
            let mut ctx = this.ctx.lock().unwrap();
            // 将过滤器函数存入 registry，以便后续调用
            match filter {
                mlua::Value::Function(f) => {
                    ctx.event_filter = Some(lua.create_registry_value(f)?);
                }
                mlua::Value::Nil => {
                    // 传入 nil 表示清除过滤器
                    if let Some(key) = ctx.event_filter.take() {
                        let _ = lua.remove_registry_value(key);
                    }
                }
                _ => {
                    return Err(mlua::Error::RuntimeError(
                        "setEventFilter expects a function or nil".to_string(),
                    ));
                }
            }
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

        // e:setMasterVolume(volume)
        methods.add_method("setMasterVolume", |_lua, this, volume: f32| {
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.set_master_volume(volume);
            Ok(())
        });

        // e:setBgmVolume(volume)
        methods.add_method("setBgmVolume", |_lua, this, volume: f32| {
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.set_bgm_volume(volume);
            Ok(())
        });

        // e:setSeVolume(volume)
        methods.add_method("setSeVolume", |_lua, this, volume: f32| {
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.set_se_volume(volume);
            Ok(())
        });

        // e:setVoiceVolume(volume)
        methods.add_method("setVoiceVolume", |_lua, this, volume: f32| {
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.set_voice_volume(volume);
            Ok(())
        });

        // e:exit() — request game exit
        methods.add_method("exit", |_lua, this, _: ()| {
            let mut ctx = this.ctx.lock().unwrap();
            ctx.tag_queue
                .push(("exit".to_string(), std::collections::HashMap::new()));
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

        // e:bindSurface(key)
        methods.add_method("bindSurface", |_lua, this, key: mlua::Value| {
            let key = lua_value_to_key(&key);
            let ctx = this.ctx.lock().unwrap();
            ctx.callbacks.bind_surface(&key);
            Ok(())
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

/// 随机整数（非负 31 位），对应 Artemis 的 `e:random()`。
///
/// 注意：游戏脚本里清一色是 `e:random() % N + 1` 的整数取模用法（见
/// sysvo/macro/user/config 等），因此必须返回**整数**而非 [0,1) 浮点，
/// 否则 `% N` 会得到小数、`tbl[小数]` 取到 nil，触发
/// "attempt to get length of field '?'" 之类崩溃。
fn rand_i64() -> i64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static STATE: AtomicU64 = AtomicU64::new(0);
    // 用毫秒时间播种一次，之后靠原子计数推进，保证同毫秒内连续调用也不同。
    let prev = STATE.load(Ordering::Relaxed);
    let seed = if prev == 0 { now_millis() as u64 } else { prev };
    let x = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    STATE.store(x, Ordering::Relaxed);
    // 取高位做 31 位非负整数
    ((x >> 33) & 0x7fff_ffff) as i64
}

/// 初始化 Lua 环境，注入 engine 对象
pub fn init_lua_engine_api(lua: &Lua, ctx: Arc<Mutex<EngineContext>>) -> LuaResult<()> {
    let engine = EngineApi::new(ctx);
    lua.globals().set("__engine", engine)?;
    Ok(())
}
