//! 主解释器（迭代版本）
//!
//! ASB 脚本解释器的核心实现，使用迭代而非递归来避免栈溢出。

use crate::error::{Error, Result};
use crate::event::{CallbackResult, Event, EventCallback, ScriptLoader, default_callback};
use crate::lua_engine::{DefaultEngineCallbacks, EngineContext};
use crate::script::{Instruction, Script};
use crate::tags::{ExecutionContext, TagRegistry, TagResult};
use crate::variable::{Value, VariableStore};
use mlua::Lua;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Artemis 兼容：字符串算术强制转换。
///
/// 大量游戏脚本（如 mulpos）依赖 `"off" * 5 → 0`、`"100" + 20 → 120` 等行为。
/// 标准 Lua 5.1 会在非数字字符串上崩溃，而 Artemis 的定制 Lua 运行时会做隐式
/// tonumber 转换并将无效字符串视为 0。
const ARITHMETIC_COERCION_CODE: &str = r#"
    local function _artemis_coerce(a, b, op)
        local an = tonumber(a)
        local bn = tonumber(b)
        if an and bn then
            return op(an, bn)
        elseif an then
            return op(an, 0)
        elseif bn then
            return op(0, bn)
        else
            return op(0, 0)
        end
    end
    local sm = getmetatable("")
    if sm then
        sm.__mul = function(a, b) return _artemis_coerce(a, b, function(x, y) return x * y end) end
        sm.__add = function(a, b) return _artemis_coerce(a, b, function(x, y) return x + y end) end
        sm.__sub = function(a, b) return _artemis_coerce(a, b, function(x, y) return x - y end) end
        sm.__div = function(a, b) return _artemis_coerce(a, b, function(x, y) return x / y end) end
        sm.__mod = function(a, b)
            local an = tonumber(a); local bn = tonumber(b)
            if an and bn then return an % bn else return 0 end
        end
        sm.__unm = function(a) return -(tonumber(a) or 0) end
        sm.__pow = function(a, b)
            local an = tonumber(a); local bn = tonumber(b)
            if an and bn then return an ^ bn else return 0 end
        end
    end
"#;

/// 调用栈帧
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// 脚本名
    pub script: String,
    /// 返回行号
    pub return_line: usize,
}

#[derive(Debug, Clone)]
pub struct QueuedTagDrain {
    pub wait: Option<Event>,
    pub saw_return: bool,
    pub changed_position: bool,
}

/// 解释器配置
///
/// 包含从 system.ini 中读取的环境变量。
/// 使用方（游戏引擎）负责解析 system.ini，
/// 并将相关配置填入此结构体后传给解释器。
#[derive(Debug, Clone)]
pub struct InterpreterConfig {
    /// 脚本字符编码（默认 UTF-8，可设为 SHIFT_JIS）
    pub encoding: &'static encoding_rs::Encoding,

    /// 舞台宽度（WIDTH）
    pub stage_width: u32,
    /// 舞台高度（HEIGHT）
    pub stage_height: u32,
    /// 帧率（FPS）
    pub fps: u32,

    /// 是否无边框窗口（FRAMELESS）
    pub frameless: bool,
    /// 是否可调整窗口大小（RESIZABLE）
    pub resizable: bool,
    /// 是否固定宽高比（FIXED_ASPECT_RATIO）
    pub fixed_aspect_ratio: bool,
    /// 是否裁剪舞台溢出部分（SIDECUT）
    pub sidecut: bool,
    /// 侧边填充图片路径（SIDE_PICTURE）
    pub side_picture: Option<String>,

    /// 是否启用节能模式（POWER_SAVING）
    pub power_saving: bool,
    /// 是否禁用存档（NO_SAVE）
    pub no_save: bool,

    /// 存档路径（SAVEPATH）
    pub savepath: Option<String>,
    /// 数据文件夹路径（s.datapath）
    pub datapath: Option<String>,
    /// 游戏标题（用于窗口标题等）
    pub title: Option<String>,

    /// 防止多重启动的标识符（PREVENT_MULTIPLE_PROCESS）
    pub process_id: Option<String>,

    /// 其他自定义环境变量（供脚本通过 s.* 访问）
    pub env: HashMap<String, String>,

    /// 目标平台标识（小写：windows / android / ios / wasm 等）。
    /// 决定 `[var system="os"]` 返回值——解释器面向某一目标平台运行，而非宿主机器，
    /// 故不能用编译期 `cfg!(target_os)`。脚本据此选择机种分支。
    pub platform: String,
}

impl Default for InterpreterConfig {
    fn default() -> Self {
        Self {
            encoding: encoding_rs::UTF_8,
            stage_width: 640,
            stage_height: 480,
            fps: 60,
            frameless: false,
            resizable: false,
            fixed_aspect_ratio: false,
            sidecut: false,
            side_picture: None,
            power_saving: false,
            no_save: false,
            savepath: None,
            datapath: None,
            title: None,
            process_id: None,
            env: HashMap::new(),
            platform: "windows".to_string(),
        }
    }
}

/// 执行结果
#[derive(Debug)]
pub enum ExecutionResult {
    /// 执行完成
    Completed,
    /// 等待用户输入
    Wait(Event),
    /// 调用外部脚本
    CallScript {
        /// 脚本文件
        file: String,
        /// 标签名
        label: String,
    },
    /// 跳转到其他脚本
    JumpScript {
        /// 脚本文件
        file: String,
        /// 标签名
        label: String,
    },
}

/// ASB 脚本解释器
pub struct Interpreter {
    /// 配置
    config: InterpreterConfig,
    /// 已加载的脚本
    scripts: HashMap<String, Script>,
    /// 变量存储
    ///
    /// 用 `Arc<Mutex<_>>` 持有，以便与 [`EngineContext`] 共享同一份变量，使 Lua 中的
    /// `e:var(name)` 能读取解释器写入的变量。
    variables: Arc<Mutex<VariableStore>>,
    /// Lua 上下文
    lua: Lua,
    /// 标签处理器注册表
    tag_registry: TagRegistry,
    /// 当前执行的脚本
    current_script: Option<String>,
    /// 当前执行行号
    current_line: usize,
    /// 调用栈
    call_stack: Vec<CallFrame>,
    /// 脚本加载器（文本）
    script_loader: Option<ScriptLoader>,
    /// 脚本文件加载器（二进制，支持自动检测）
    ///
    /// 用 `Arc` 持有，便于与 [`EngineContext::file_reader`] 共享同一个加载器，
    /// 使 `e:include` 能读取项目文件。
    file_loader: Option<crate::lua_engine::FileReader>,
    /// 事件回调
    callback: EventCallback,
    /// Lua engine 上下文
    engine_ctx: Arc<Mutex<EngineContext>>,
    /// 上一次返回的 `Wait` 是否来自**排队标签**（经 flush_tag_queue 抽干时产生）。
    ///
    /// 排队标签在 `[calllua]` 执行期间由 Lua 经 `e:tag{}` 排入，flush 发生在 step
    /// 循环顶部，此时 `current_line` 已指向**下一条尚未执行**的指令。若宿主在 Wait
    /// 释放后照常调用 `advance_line()`，会越过这条待执行指令——典型表现是 estag 链
    /// 末尾的 `eqwait` 排队 `[wait]` 发出定时等待后，advance 把行号从 `[return]`
    /// 跳到相邻的 `*movie_play [stop]`，导致 brandlogo 卡死。故此处记录来源，
    /// 让 `advance_line()` 对排队来源的 Wait 退化为空操作。
    last_wait_from_queue: bool,
    last_flush_saw_return: bool,
    last_flush_changed_position: bool,
}

fn json_to_lua_value(lua: &Lua, value: serde_json::Value) -> mlua::Result<mlua::Value> {
    match value {
        serde_json::Value::Null => Ok(mlua::Value::Nil),
        serde_json::Value::Bool(value) => Ok(mlua::Value::Boolean(value)),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(mlua::Value::Integer(value))
            } else if let Some(value) = value.as_u64() {
                if value <= i64::MAX as u64 {
                    Ok(mlua::Value::Integer(value as i64))
                } else {
                    Ok(mlua::Value::Number(value as f64))
                }
            } else {
                Ok(mlua::Value::Number(value.as_f64().unwrap_or(0.0)))
            }
        }
        serde_json::Value::String(value) => Ok(mlua::Value::String(lua.create_string(&value)?)),
        serde_json::Value::Array(values) => {
            let table = lua.create_table()?;
            for (index, value) in values.into_iter().enumerate() {
                table.set((index + 1) as i64, json_to_lua_value(lua, value)?)?;
            }
            Ok(mlua::Value::Table(table))
        }
        serde_json::Value::Object(values) => {
            let table = lua.create_table()?;
            for (key, value) in values {
                let value = json_to_lua_value(lua, value)?;
                if let Some(key) = parse_canonical_integer_key(&key) {
                    table.set(key, value)?;
                } else {
                    table.set(key, value)?;
                }
            }
            Ok(mlua::Value::Table(table))
        }
    }
}

fn lua_value_to_json_value(value: mlua::Value, depth: usize) -> mlua::Result<serde_json::Value> {
    if depth > 128 {
        return Err(mlua::Error::external(
            "JSON encode: Lua table nesting too deep",
        ));
    }

    match value {
        mlua::Value::Nil => Ok(serde_json::Value::Null),
        mlua::Value::Boolean(value) => Ok(serde_json::Value::Bool(value)),
        mlua::Value::Integer(value) => Ok(serde_json::Value::Number(value.into())),
        mlua::Value::Number(value) => serde_json::Number::from_f64(value)
            .map(serde_json::Value::Number)
            .ok_or_else(|| mlua::Error::external("JSON encode: invalid number")),
        mlua::Value::String(value) => Ok(serde_json::Value::String(value.to_string_lossy())),
        mlua::Value::Table(table) => {
            let mut object = serde_json::Map::new();
            for pair in table.pairs::<mlua::Value, mlua::Value>() {
                let (key, value) = pair?;
                if let Some(key) = lua_key_to_json_key(key) {
                    object.insert(key, lua_value_to_json_value(value, depth + 1)?);
                }
            }
            Ok(serde_json::Value::Object(object))
        }
        _ => Ok(serde_json::Value::Null),
    }
}

fn lua_key_to_json_key(key: mlua::Value) -> Option<String> {
    match key {
        mlua::Value::String(value) => Some(value.to_string_lossy()),
        mlua::Value::Integer(value) => Some(value.to_string()),
        mlua::Value::Number(value) if value.fract() == 0.0 => Some((value as i64).to_string()),
        mlua::Value::Number(value) => Some(value.to_string()),
        mlua::Value::Boolean(value) => Some(if value { "true" } else { "false" }.to_string()),
        _ => None,
    }
}

fn parse_canonical_integer_key(key: &str) -> Option<i64> {
    let value = key.parse::<i64>().ok()?;
    if value.to_string() == key {
        Some(value)
    } else {
        None
    }
}

impl Interpreter {
    /// 创建新的解释器实例
    pub fn new(config: InterpreterConfig) -> Self {
        let lua = Lua::new();

        // 注入 Pluto 序列化——使用 serde_json 作为持久化后端。
        // Artemis 脚本调用 pluto.persist(refs, tbl) / pluto.unpersist(refs, str)
        // 来序列化 Lua 表。这里用 JSON 替代原版的 Pluto 二进制格式，功能等价。
        let pluto_code = r#"
            pluto = pluto or {}
            function pluto.persist(refs, tbl)
                local ok, json = pcall(function() return __art3m1s_json_encode(tbl) end)
                if ok then return json else return "" end
            end
            function pluto.unpersist(refs, str)
                if type(str) ~= "string" or str == "" then return {} end
                local ok, tbl = pcall(function() return __art3m1s_json_decode(str) end)
                if ok and type(tbl) == "table" then return tbl else return {} end
            end
        "#;

        if let Err(e) = lua.load(pluto_code).exec() {
            eprintln!("警告: 注入 Pluto 桩实现失败: {:?}", e);
        }

        // 注册 JSON 编解码函数
        let encode = lua.create_function(|lua, value: mlua::Value| {
            let value = lua_value_to_json_value(value, 0)?;
            let json = serde_json::to_string(&value)
                .map_err(|e| mlua::Error::external(format!("JSON encode: {e}")))?;
            Ok(mlua::Value::String(lua.create_string(json)?))
        });
        let decode = lua.create_function(|lua, json_str: mlua::String| {
            let s: String = json_str.to_string_lossy();
            if s.is_empty() {
                return Ok(mlua::Value::Table(lua.create_table()?));
            }
            let value: serde_json::Value = serde_json::from_str(&s)
                .map_err(|e| mlua::Error::external(format!("JSON decode: {e}")))?;
            json_to_lua_value(lua, value)
        });
        if let Err(e) = encode.and_then(|f| lua.globals().set("__art3m1s_json_encode", f)) {
            eprintln!("警告: 注册 JSON encode 失败: {:?}", e);
        }
        if let Err(e) = decode.and_then(|f| lua.globals().set("__art3m1s_json_decode", f)) {
            eprintln!("警告: 注册 JSON decode 失败: {:?}", e);
        }

        // 注入 Artemis 兼容的算术强制转换：字符串自动转数字，非数字字符串视为 0。
        // Artemis 引擎使用定制 Lua，允许 "off" * 5 → 0、"100" + 20 → 120 等。
        // 标准 Lua 5.1 会在这种运算上崩溃，而大量游戏脚本依赖此行为。
        if let Err(e) = lua.load(ARITHMETIC_COERCION_CODE).exec() {
            eprintln!("警告: 注入算术强制转换失败: {:?}", e);
        }

        // 注入探针：注册全局错误追踪器，记录最后发生的 Lua 错误及其调用栈。
        if let Err(e) = lua
            .load(
                r#"
            __artemis_last_error = nil
            function __artemis_traceback(msg)
                __artemis_last_error = msg .. "\n" .. debug.traceback("", 2)
                return msg
            end
        "#,
            )
            .exec()
        {
            eprintln!("警告: 注入错误探针失败: {:?}", e);
        }

        let variables = Arc::new(Mutex::new(VariableStore::new()));
        {
            let mut vars = variables.lock().unwrap();
            // 把目标平台写入变量存储，供 [var system="os"] 读取（见 InterpreterConfig::platform）。
            vars.set_platform(config.platform.clone());
            // 引擎提供的系统常量（s.*）。它们不是存档状态，而是运行环境信息，脚本启动
            // 时直接读取。缺省会让 init.lua 里的数值比较（如 windowsversion 兼容性检查）
            // 对 nil 求值而崩溃，故在此种入合理默认值。
            vars.set(
                "s.engineversion",
                crate::variable::Value::String("4.00".to_string()),
            );
            vars.set(
                "s.windowsversion",
                crate::variable::Value::String("10.0".to_string()),
            );
            // 舞台尺寸供 get_layer_info 等系统查询使用
            vars.set(
                "s.screen_width",
                crate::variable::Value::Int(config.stage_width as i64),
            );
            vars.set(
                "s.screen_height",
                crate::variable::Value::Int(config.stage_height as i64),
            );
            // 存档路径 / 数据路径：脚本用 e:var("s.savepath") 读取。
            // 缺省会让 saveload_init 拼 nil 崩溃。
            let savepath = config
                .savepath
                .clone()
                .unwrap_or_else(|| "save".to_string());
            vars.set("s.savepath", crate::variable::Value::String(savepath));
            if let Some(dp) = &config.datapath {
                vars.set("s.datapath", crate::variable::Value::String(dp.clone()));
            }
        }
        let mut engine_ctx_inner = EngineContext::new(Box::new(DefaultEngineCallbacks));
        // 共享同一份变量存储给 engine 上下文，使 e:var 能读到解释器写入的变量。
        engine_ctx_inner.variables = Some(Arc::clone(&variables));
        let engine_ctx = Arc::new(Mutex::new(engine_ctx_inner));
        let _ = crate::lua_engine::init_lua_engine_api(&lua, Arc::clone(&engine_ctx));

        Self {
            config,
            scripts: HashMap::new(),
            variables,
            lua,
            tag_registry: TagRegistry::new(),
            current_script: None,
            current_line: 0,
            call_stack: Vec::new(),
            script_loader: None,
            file_loader: None,
            callback: Box::new(default_callback),
            engine_ctx,
            last_wait_from_queue: false,
            last_flush_saw_return: false,
            last_flush_changed_position: false,
        }
    }

    /// 设置自定义 Lua engine 回调
    ///
    /// 宿主应用可以通过此方法注入自定义回调，
    /// 响应 Lua 脚本中 engine 对象（`e`）的方法调用。
    pub fn set_engine_callbacks(&mut self, callbacks: Box<dyn crate::lua_engine::EngineCallbacks>) {
        self.engine_ctx.lock().unwrap().callbacks = callbacks;
    }

    /// 获取 Lua engine 上下文
    pub fn engine_context(&self) -> &Arc<Mutex<EngineContext>> {
        &self.engine_ctx
    }

    /// 加载脚本（从文本）
    pub fn load_script(&mut self, name: &str, content: &str) -> Result<()> {
        let script = Script::parse(name, content)?;
        self.scripts.insert(name.to_string(), script);
        Ok(())
    }

    /// 加载脚本（从 ASB 二进制数据）
    pub fn load_asb(&mut self, name: &str, data: &[u8]) -> Result<()> {
        let text = asb_decrypt::decode_asb_to_string(data)?;
        self.load_script(name, &text)
    }

    /// 智能加载脚本（自动检测文件格式）
    ///
    /// 根据文件魔数自动判断是文本格式（.iet, .ast）还是二进制格式（.asb）：
    /// - 如果以 `ASB\0` 开头，则作为二进制 ASB 文件解密
    /// - 否则作为文本文件直接解析
    ///
    /// 支持的扩展名：
    /// - `.iet` - 文本格式（未加密）
    /// - `.ast` - 文本格式（未加密）
    /// - `.asb` - 二进制格式（加密）
    pub fn load_file(&mut self, name: &str, data: &[u8]) -> Result<()> {
        // 检查是否是 ASB 二进制格式（魔数: ASB\0）
        if data.len() >= 4 && &data[0..4] == b"ASB\x00" {
            self.load_asb(name, data)
        } else {
            // 作为文本处理
            let text = String::from_utf8_lossy(data);
            self.load_script(name, &text)
        }
    }

    /// 设置脚本加载器（文本格式）
    pub fn set_script_loader(&mut self, loader: ScriptLoader) {
        self.script_loader = Some(loader);
    }

    /// 设置脚本文件加载器（支持文本和二进制格式的自动检测）
    ///
    /// 这是推荐的方式，能够自动处理 .iet/.ast（文本）和 .asb（二进制）文件。
    ///
    /// 同一个加载器会被共享到 [`EngineContext`]，使 Lua 中的 `e:include` 能读取
    /// 并执行项目内的 `.lua`/数据文件。
    pub fn set_file_loader(&mut self, loader: crate::event::ScriptFileLoader) {
        // ScriptFileLoader 是 Box；转成 Arc 以便与 engine_ctx 共享同一闭包。
        let shared: crate::lua_engine::FileReader = Arc::from(loader);
        self.engine_ctx.lock().unwrap().file_reader = Some(Arc::clone(&shared));
        self.file_loader = Some(shared);
    }

    /// 加载外部脚本（自动检测格式）
    pub fn load_external_script(&mut self, file: &str) -> Result<()> {
        if self.scripts.contains_key(file) {
            return Ok(());
        }

        // 优先使用文件加载器（支持二进制和文本）
        if let Some(loader) = &self.file_loader {
            let data = loader(file)?;
            return self.load_file(file, &data);
        }

        // 回退到文本加载器
        if let Some(loader) = &self.script_loader {
            let content = loader(file)?;
            return self.load_script(file, &content);
        }

        Err(Error::ScriptNotFound(file.to_string()))
    }

    /// 设置起始标签并开始执行
    pub fn start(&mut self, script: &str, label: &str) -> Result<()> {
        // 确保脚本已加载
        if !self.scripts.contains_key(script) {
            self.load_external_script(script)?;
        }

        // 查找标签
        let script_obj = self
            .scripts
            .get(script)
            .ok_or_else(|| Error::ScriptNotFound(script.to_string()))?;

        let line = script_obj
            .get_label_line(label)
            .ok_or_else(|| Error::LabelNotFound(label.to_string()))?;

        self.current_script = Some(script.to_string());
        self.current_line = line;
        self.call_stack.clear();

        Ok(())
    }

    /// 执行到下一个等待点（迭代版本，避免栈溢出）
    pub fn step(&mut self) -> Result<ExecutionResult> {
        loop {
            // 先抽干 Lua 通过 e:tag{} 排队的标签（如图层操作），它们由上一条
            // [calllua]/[lua] 产生，必须走标签管线才能发出对应事件。
            if let Some(result) = self.flush_tag_queue()? {
                return Ok(result);
            }

            // 走到这里说明队列已抽干，接下来执行的是脚本流里的内联指令；它若产生
            // Wait，current_line 指向的就是该 Wait 指令本身，宿主需要 advance_line()
            // 越过它。故在此把来源标记清为「非排队」。
            self.last_wait_from_queue = false;

            let script_name = match &self.current_script {
                Some(name) => name.clone(),
                None => return Ok(ExecutionResult::Completed),
            };

            let script = match self.scripts.get(&script_name) {
                Some(s) => s,
                None => return Ok(ExecutionResult::Completed),
            };

            if self.current_line >= script.len() {
                return Ok(ExecutionResult::Completed);
            }

            let instruction = script.instructions[self.current_line].clone();

            if std::env::var("ASB_TRACE_STEP").is_ok() {
                eprintln!(
                    "[step] {}:{} tag={} fn={:?}",
                    script_name,
                    self.current_line,
                    instruction.tag,
                    instruction.get("function")
                );
            }

            // 处理剧情文本
            if instruction.tag == "__text" {
                let text = instruction.get("text").unwrap_or("").to_string();
                let result = (self.callback)(Event::ScenarioText {
                    content: text.clone(),
                    inline: false,
                });
                match result {
                    CallbackResult::Continue => {
                        self.current_line += 1;
                        continue;
                    }
                    CallbackResult::Pause => {
                        return Ok(ExecutionResult::Wait(Event::ScenarioText {
                            content: text,
                            inline: false,
                        }));
                    }
                    CallbackResult::Abort => {
                        return Err(Error::Aborted);
                    }
                }
            }

            // 处理 Lua 代码块
            if instruction.tag == "__lua_block" {
                let code = instruction.get("code").unwrap_or("");
                // 执行 Lua 代码
                if let Err(e) = self.lua.load(code).exec() {
                    return Err(Error::LuaError(e));
                }
                self.current_line += 1;
                continue;
            }

            // 执行标签
            let tag_result = self.execute_tag(&instruction)?;

            match tag_result {
                TagResult::Continue => {
                    self.current_line += 1;
                    continue;
                }
                TagResult::Jump(line) => {
                    self.current_line = line;
                    continue;
                }
                TagResult::Call {
                    file,
                    label,
                    return_line,
                    return_script,
                } => {
                    // 压入调用栈
                    self.call_stack.push(CallFrame {
                        script: return_script.clone(),
                        return_line,
                    });

                    // 跳转到目标
                    if let Some(target_file) = file {
                        // 跨脚本调用
                        self.load_external_script(&target_file)?;
                        let target_script = self
                            .scripts
                            .get(&target_file)
                            .ok_or_else(|| Error::ScriptNotFound(target_file.clone()))?;
                        let target_line = target_script
                            .get_label_line(&label)
                            .ok_or_else(|| Error::LabelNotFound(label.clone()))?;

                        self.current_script = Some(target_file.clone());
                        self.current_line = target_line;
                        continue;
                    } else {
                        // 同脚本调用
                        let script = self.scripts.get(&return_script).unwrap();
                        let line = script
                            .get_label_line(&label)
                            .ok_or_else(|| Error::LabelNotFound(label.clone()))?;
                        self.current_line = line;
                        continue;
                    }
                }
                TagResult::Return => {
                    if let Some(frame) = self.call_stack.pop() {
                        self.current_script = Some(frame.script);
                        self.current_line = frame.return_line;
                        continue;
                    } else {
                        return Ok(ExecutionResult::Completed);
                    }
                }
                TagResult::Wait(event) => {
                    let result = (self.callback)(event.clone());
                    match result {
                        CallbackResult::Continue => {
                            self.current_line += 1;
                            continue;
                        }
                        CallbackResult::Pause => {
                            return Ok(ExecutionResult::Wait(event));
                        }
                        CallbackResult::Abort => {
                            return Err(Error::Aborted);
                        }
                    }
                }
                TagResult::Emit(event) => {
                    let result = (self.callback)(event.clone());
                    match result {
                        CallbackResult::Continue => {
                            self.current_line += 1;
                            continue;
                        }
                        CallbackResult::Pause => {
                            return Ok(ExecutionResult::Wait(event));
                        }
                        CallbackResult::Abort => {
                            return Err(Error::Aborted);
                        }
                    }
                }
                TagResult::EmitMany(events) => {
                    for event in events {
                        let result = (self.callback)(event.clone());
                        match result {
                            CallbackResult::Continue => {}
                            CallbackResult::Pause => {
                                return Ok(ExecutionResult::Wait(event));
                            }
                            CallbackResult::Abort => {
                                return Err(Error::Aborted);
                            }
                        }
                    }
                    self.current_line += 1;
                    continue;
                }
                TagResult::Dynamic(inner_instruction) => {
                    // 动态执行另一条指令（用于 tag 标签）
                    let inner_result = self.execute_tag(&inner_instruction)?;
                    // 处理内部指令的结果（不增加行号，因为外层会处理）
                    match inner_result {
                        TagResult::Continue => {
                            self.current_line += 1;
                            continue;
                        }
                        TagResult::Jump(line) => {
                            self.current_line = line;
                            continue;
                        }
                        TagResult::Wait(event) => {
                            let result = (self.callback)(event.clone());
                            match result {
                                CallbackResult::Continue => {
                                    self.current_line += 1;
                                    continue;
                                }
                                CallbackResult::Pause => {
                                    return Ok(ExecutionResult::Wait(event));
                                }
                                CallbackResult::Abort => {
                                    return Err(Error::Aborted);
                                }
                            }
                        }
                        TagResult::Emit(event) => {
                            let result = (self.callback)(event.clone());
                            match result {
                                CallbackResult::Continue => {
                                    self.current_line += 1;
                                    continue;
                                }
                                CallbackResult::Pause => {
                                    return Ok(ExecutionResult::Wait(event));
                                }
                                CallbackResult::Abort => {
                                    return Err(Error::Aborted);
                                }
                            }
                        }
                        TagResult::EmitMany(events) => {
                            for event in events {
                                let result = (self.callback)(event.clone());
                                match result {
                                    CallbackResult::Continue => {}
                                    CallbackResult::Pause => {
                                        return Ok(ExecutionResult::Wait(event));
                                    }
                                    CallbackResult::Abort => {
                                        return Err(Error::Aborted);
                                    }
                                }
                            }
                            self.current_line += 1;
                            continue;
                        }
                        TagResult::Dynamic(_) => {
                            // 不支持嵌套动态标签
                            return Err(Error::RuntimeError {
                                line: self.current_line,
                                message: "不支持嵌套的 tag 标签".to_string(),
                            });
                        }
                        other => {
                            // Call/Return 等直接处理
                            self.current_line += 1;
                            // 将结果返回给外层处理
                            match other {
                                TagResult::Call {
                                    file,
                                    label,
                                    return_line,
                                    return_script,
                                } => {
                                    self.call_stack.push(CallFrame {
                                        script: return_script,
                                        return_line,
                                    });
                                    if let Some(target_file) = file {
                                        self.load_external_script(&target_file)?;
                                        let target_script =
                                            self.scripts.get(&target_file).ok_or_else(|| {
                                                Error::ScriptNotFound(target_file.clone())
                                            })?;
                                        let target_line = target_script
                                            .get_label_line(&label)
                                            .ok_or_else(|| Error::LabelNotFound(label.clone()))?;
                                        self.current_script = Some(target_file);
                                        self.current_line = target_line;
                                    } else {
                                        let script_name =
                                            self.current_script.clone().unwrap_or_default();
                                        let script = self.scripts.get(&script_name).unwrap();
                                        let line = script
                                            .get_label_line(&label)
                                            .ok_or_else(|| Error::LabelNotFound(label.clone()))?;
                                        self.current_line = line;
                                    }
                                    continue;
                                }
                                TagResult::Return => {
                                    if let Some(frame) = self.call_stack.pop() {
                                        self.current_script = Some(frame.script);
                                        self.current_line = frame.return_line;
                                        continue;
                                    } else {
                                        return Ok(ExecutionResult::Completed);
                                    }
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                }
            }
        }
    }

    /// 抽干 Lua 通过 `e:tag{}` / `e:enqueueTag{}` 排入的标签队列。
    ///
    /// `[calllua]`/`[lua]` 执行时，Lua 脚本会调用 `e:tag{name="lyc",...}` 之类把
    /// 图层等标签推入 [`EngineContext::tag_queue`]。这些标签必须走正常的标签管线
    /// [`execute_tag`](Self::execute_tag) 才能产出对应事件（如 [`Event::Layer`]）。
    /// 此前队列只被写入、从无人消费，导致 Lua 驱动的图层操作全部丢失。
    ///
    /// 每次只取队首一个并执行，这样回调返回 `Pause` 时，剩余标签仍留在队列里，
    /// 下次 `run()` 重入 `step` 可继续处理。返回 `Some(result)` 表示需立即从 `step`
    /// 返回（暂停或中止），`None` 表示队列已清空、可继续正常执行。
    fn flush_tag_queue(&mut self) -> Result<Option<ExecutionResult>> {
        loop {
            let queued = {
                let mut ctx = self.engine_ctx.lock().unwrap();
                if ctx.tag_queue.is_empty() {
                    None
                } else {
                    Some(ctx.tag_queue.remove(0))
                }
            };
            let Some((tag, params)) = queued else {
                return Ok(None);
            };

            let instruction = Instruction {
                tag,
                params,
                line: self.current_line,
            };

            if std::env::var("ASB_TRACE_FLUSH").is_ok() {
                eprintln!(
                    "[flush] tag={} fn={:?} params={:?}",
                    instruction.tag,
                    instruction.get("function"),
                    instruction.params.keys().collect::<Vec<_>>()
                );
            }

            // `tag` 标签自身会返回 Dynamic，需再展开一层拿到真正的指令。
            let mut result = self.execute_tag(&instruction)?;
            if let TagResult::Dynamic(inner) = result {
                result = self.execute_tag(&inner)?;
            }

            match result {
                TagResult::Continue => continue,
                TagResult::Emit(event) | TagResult::Wait(event) => {
                    match (self.callback)(event.clone()) {
                        CallbackResult::Continue => continue,
                        CallbackResult::Pause => {
                            // 这个 Wait 来自排队标签，current_line 已指向下一条待执行
                            // 指令；记录来源，使 advance_line() 退化为空操作。
                            self.last_wait_from_queue = true;
                            return Ok(Some(ExecutionResult::Wait(event)));
                        }
                        CallbackResult::Abort => return Err(Error::Aborted),
                    }
                }
                TagResult::EmitMany(events) => {
                    for event in events {
                        match (self.callback)(event.clone()) {
                            CallbackResult::Continue => {}
                            CallbackResult::Pause => {
                                self.last_wait_from_queue = true;
                                return Ok(Some(ExecutionResult::Wait(event)));
                            }
                            CallbackResult::Abort => return Err(Error::Aborted),
                        }
                    }
                    continue;
                }
                // Lua 也会通过 eqtag/enqueueTag 排入控制流标签，最典型的是
                // `eqtag{"jump", file=..., label="game_start"}`（跨脚本跳转返回
                // TagResult::Call）。必须落实位置变更，否则 boot 推进不到 game_start。
                // 改动 current_script/current_line 后继续抽干队列；flush 返回后
                // step 主循环会从新位置读取指令。
                TagResult::Jump(line) => {
                    self.last_flush_changed_position = true;
                    self.current_line = line;
                    // 继续抽干剩余标签而非立即返回——排在 jump 之后的 calllua
                    // 等函数调用仍有效（典型：fn.push 的 jump 和按钮点击 handler
                    // 先后入队，jump 先于 handler 被抽到，若此时 return 则 handler
                    // 被延迟到 jump 后的脚本上下文才执行，导致 dialog 返回值丢失）。
                    continue;
                }
                TagResult::Call {
                    file,
                    label,
                    return_line: _,
                    return_script,
                } => {
                    self.last_flush_changed_position = true;
                    // 排队 call 与内联 call 的返回语义不同：
                    // 内联 call 时 `self.current_line` 指向 call 指令本身，handler
                    // 用 `current_line + 1` 让 return 落到下一条指令是对的。
                    // 但排队 call（Lua 经 enqueueTag 排入）是在 step 循环顶部抽干
                    // 队列时执行的，此时 `self.current_line` 已经指向**尚未执行**的
                    // 下一条指令。若仍用 handler 的 `current_line + 1`，return 会跳过
                    // 这条待执行指令。因此排队 call 必须返回到 `self.current_line`
                    // 本身，把它执行掉。
                    // 典型案例：system_initialize 在 `[calllua]` 中 enqueueTag 三个
                    // `[call]` 缓存系统脚本，return 必须回到紧随其后的
                    // `[calllua system_starting]`，否则 boot 推进不到 title。
                    let return_line = self.current_line;
                    if std::env::var("ASB_TRACE_FLUSH").is_ok() {
                        eprintln!(
                            "[flush-call] file={:?} label={} return_line={} return_script={}",
                            file, label, return_line, return_script
                        );
                    }
                    self.call_stack.push(CallFrame {
                        script: return_script.clone(),
                        return_line,
                    });
                    if let Some(target_file) = file {
                        self.load_external_script(&target_file)?;
                        let target_line = self
                            .scripts
                            .get(&target_file)
                            .ok_or_else(|| Error::ScriptNotFound(target_file.clone()))?
                            .get_label_line(&label)
                            .ok_or_else(|| Error::LabelNotFound(label.clone()))?;
                        self.current_script = Some(target_file);
                        self.current_line = target_line;
                    } else {
                        let line = self
                            .scripts
                            .get(&return_script)
                            .ok_or_else(|| Error::ScriptNotFound(return_script.clone()))?
                            .get_label_line(&label)
                            .ok_or_else(|| Error::LabelNotFound(label.clone()))?;
                        self.current_line = line;
                    }
                    // 继续抽干剩余标签——calllua 等函数调用在跨脚本跳转后仍然有效。
                    continue;
                }
                TagResult::Return => {
                    self.last_flush_saw_return = true;
                    self.last_flush_changed_position = true;
                    if let Some(frame) = self.call_stack.pop() {
                        self.current_script = Some(frame.script);
                        self.current_line = frame.return_line;
                    }
                    continue;
                }
                // Dynamic 已在上面展开一层；理论上不会再出现，安全忽略。
                TagResult::Dynamic(_) => continue,
            }
        }
    }

    /// 执行单个标签
    fn execute_tag(&mut self, instruction: &Instruction) -> Result<TagResult> {
        let script_name = self.current_script.clone().unwrap_or_default();
        let current_line = self.current_line;

        // calllua 会同步执行 Lua 函数，而该函数可能回调 e:var（再次锁 variables）。
        // 若在持有 variables 锁期间执行它会自锁死，故像 __lua_block 一样特判：
        // 不持 variables 锁、直接调用。CallLuaHandler 本身也未使用 ctx.variables。
        if instruction.tag == "calllua" {
            let function_name = instruction.get("function").unwrap_or("");
            if function_name.is_empty() {
                return Err(Error::RuntimeError {
                    line: current_line,
                    message: "calllua 缺少 function 参数".to_string(),
                });
            }
            let mut extra_params = HashMap::new();
            for (key, value) in &instruction.params {
                if key != "function" {
                    extra_params.insert(key.clone(), value.clone());
                }
            }
            // 关键：不持 variables 锁。call_lua_function 同步执行的 Lua 可能回调
            // e:var（经共享句柄再次锁 variables），持锁会在非可重入 Mutex 上自锁死。
            crate::tags::call_lua_function(&self.lua, function_name, &extra_params)?;
            return Ok(TagResult::Continue);
        }

        // 先获取 handler，避免借用冲突
        let handler_result = self.tag_registry.get(&instruction.tag);

        if handler_result.is_some() {
            // 创建上下文
            let get_script = |name: &str| -> Option<&Script> { self.scripts.get(name) };

            // 锁定共享变量存储，仅在本次标签执行期间持有。非 Lua 执行类标签不会
            // 重入 e:var，故此处持锁安全（calllua 已在上面特判，不走这里）。
            let mut vars = self.variables.lock().unwrap();
            let mut ctx = ExecutionContext {
                variables: &mut vars,
                lua: &self.lua,
                current_script: &script_name,
                current_line,
                instruction,
                get_script: &get_script,
            };

            // 获取 handler 并执行
            if let Some(handler) = self.tag_registry.get(&instruction.tag) {
                handler.execute(&mut ctx)
            } else {
                unreachable!()
            }
        } else {
            // 未注册的标签：先尝试 Lua `tags` 表分发（游戏自定义标签，如 `msgon`、
            // `delay0`、`btn_click` 等）。游戏脚本在 `system/extend/script.lua` 中通过
            // `tags.<tag> = function(e, p) ... end` 注册这些处理器。
            //
            // 注意：不能持有 variables 锁期间调用 Lua（可能回调 e:var 再次锁），
            // 故仿 calllua 特判——不持锁、直接调用。
            if let Some(_result) = self.try_dispatch_lua_tag(&instruction.tag, &instruction.params)
            {
                return Ok(TagResult::Continue);
            }
            // Lua 也未注册，回退：发出自定义事件
            Ok(TagResult::Emit(Event::Custom {
                tag: instruction.tag.clone(),
                params: instruction.params.clone(),
            }))
        }
    }

    /// 尝试把未注册标签分发给 Lua 全局 `tags` 表中的同名函数。
    ///
    /// 游戏自定义标签（如 `msgon`、`delay0`、`btn_click` 等）在
    /// `system/extend/script.lua` 中以 `tags.<tag> = function(e, p) ... end` 注册。
    /// 引擎自身不内置这些标签，而是在此处尝试 Lua 分发。
    ///
    /// 调用签名与 `calllua` 一致：`func(__engine, param_table)`。
    /// 返回 `true` 表示找到并执行了 Lua handler，`false` 表示无此 handler。
    fn try_dispatch_lua_tag(&self, tag: &str, params: &HashMap<String, String>) -> Option<()> {
        // 查找 tags.<tag> 函数，嵌套路径如 tags.msgon 走 lua 递归解析
        let func: mlua::Function = {
            let globals = self.lua.globals();
            let tags: mlua::Table = globals.get("tags").ok()?;
            let parts: Vec<&str> = tag.split('.').collect();
            let mut current: mlua::Value = tags.get(parts[0]).ok()?;
            for &part in &parts[1..] {
                current = match current {
                    mlua::Value::Table(t) => t.get(part).ok()?,
                    _ => return None,
                };
            }
            match current {
                mlua::Value::Function(f) => f,
                _ => return None,
            }
        };

        // 构造 param 表
        let param_table = match self.lua.create_table() {
            Ok(t) => t,
            Err(_) => return None,
        };
        for (k, v) in params {
            let _ = param_table.set(k.as_str(), v.as_str());
        }

        // 获取 engine 对象
        let engine: mlua::Value = self.lua.globals().get("__engine").ok()?;

        let result: mlua::Result<()> = match engine {
            mlua::Value::UserData(ud) => func.call((ud, param_table)),
            _ => func.call((param_table,)),
        };

        match result {
            Ok(_) => Some(()),
            Err(e) => {
                if std::env::var("ART3M1S_DEBUG").is_ok() {
                    eprintln!("[lua-tag] {tag} 执行失败: {e}");
                }
                None
            }
        }
    }

    /// 持续执行直到完成或等待
    pub fn run(&mut self) -> Result<ExecutionResult> {
        self.step()
    }

    /// 触发注册在 `onEnterFrame` 上的每帧回调（Artemis 约定 `e:setEventHandler{
    /// onEnterFrame="vsync"}`）。宿主应在每帧驱动一次。
    ///
    /// 该回调（如 `vsync`）承载大量周期性逻辑：清除 `flg.imageCacheStart` 加载等待
    /// 标志、键盘 edge 检测、自动模式/快进、lipsync 等。若不驱动，`imageCacheStart`
    /// 永不清除，会导致 `setonpush_calllua` 在入口直接 return，**所有按钮点击全部失效**。
    ///
    /// Lua 函数内部通过 `e:tag{}` 排队的标签留在队列里，由随后的 `run()` 抽干。
    /// 注意：不能在持有 ctx 锁时调用 Lua（会重入再次锁 ctx），故先取出 handler 名
    /// 释放锁，再调用——与 [`crate::tags::call_lua_function`] 的约束一致。
    pub fn fire_enter_frame(&mut self) -> Result<()> {
        let handler = {
            let ctx = self.engine_ctx.lock().unwrap();
            ctx.event_handlers.get("onEnterFrame").cloned()
        };
        if let Some(func) = handler {
            crate::tags::call_lua_function(self.lua(), &func, &HashMap::new())?;
        }
        Ok(())
    }

    /// 触发存档前的 `onSave` 处理器（脚本中通过 `e:setEventHandler{onSave=...}` 注册）。
    ///
    /// 该回调（如 `store`）负责把 `sys`/`gscr`/`conf` 等 Lua 表经 pluto 序列化进
    /// Artemis 变量（`fsave_pluto` → `e:tag{"var",...}`），从而让存档界面所需的
    /// `sys.saveslot` 元数据随变量一同落盘。**必须在快照变量之前调用**，且调用后
    /// 还需抽干标签队列（`[var]` 标签是排队执行的），快照才能包含这些变量。
    ///
    /// 与 [`Self::fire_enter_frame`] 同样的约束：先取出 handler 名释放锁再调用 Lua。
    pub fn fire_save_handler(&mut self) -> Result<()> {
        let handler = {
            let ctx = self.engine_ctx.lock().unwrap();
            ctx.event_handlers.get("onSave").cloned()
        };
        if let Some(func) = handler {
            crate::tags::call_lua_function(self.lua(), &func, &HashMap::new())?;
        }
        Ok(())
    }

    /// 触发读档后的 `onLoad` 处理器（脚本中通过 `e:setEventHandler{onLoad=...}` 注册）。
    ///
    /// 该回调（如 `restore`）负责把读档恢复的 Artemis 变量经 pluto 反序列化回
    /// `sys`/`gscr`/`conf`/`scr`/`log` 等 Lua 表（`loadconv` → `fload_pluto`），
    /// 否则即便变量已恢复，承载游戏态与存档槽位的 Lua 表仍是旧的。
    /// **必须在 [`Self::restore_variables`] 之后调用**。
    pub fn fire_load_handler(&mut self) -> Result<()> {
        let handler = {
            let ctx = self.engine_ctx.lock().unwrap();
            ctx.event_handlers.get("onLoad").cloned()
        };
        if let Some(func) = handler {
            crate::tags::call_lua_function(self.lua(), &func, &HashMap::new())?;
        }
        Ok(())
    }

    /// 抽干当前排队的标签（公开包装 [`Self::flush_tag_queue`]）。
    ///
    /// 供宿主在触发 `onSave`/`onLoad` 处理器后调用：`store`/`restore` 通过
    /// `e:tag{"var",...}` 把序列化结果排入标签队列，这些 `[var]` 标签返回
    /// `Continue`，一次抽干即可将变量真正写入 `VariableStore`，使随后的存档快照
    /// 或读档恢复看到完整变量。
    pub fn flush_pending_tags(&mut self) -> Result<()> {
        self.flush_tag_queue()?;
        Ok(())
    }

    /// 只抽干 Lua/事件排入的标签队列，不在队列清空后继续执行主脚本。
    pub fn drain_queued_tags_only(&mut self) -> Result<QueuedTagDrain> {
        self.last_flush_saw_return = false;
        self.last_flush_changed_position = false;
        let wait = match self.flush_tag_queue()? {
            Some(ExecutionResult::Wait(event)) => Some(event),
            Some(ExecutionResult::Completed) | None => None,
            Some(_) => None,
        };
        Ok(QueuedTagDrain {
            wait,
            saw_return: self.last_flush_saw_return,
            changed_position: self.last_flush_changed_position,
        })
    }

    /// 当前行号加一。
    ///
    /// 供宿主在 `run()` 返回 `ExecutionResult::Wait` 后调用，越过触发 Wait 的那条指令，
    /// 以便下一次 `run()` 能从下一指令继续执行（例如用户点击推进的 `[wt]` 等待）。
    ///
    /// 但若该 Wait 来自**排队标签**（见 [`Self::last_wait_from_queue`]），则
    /// `current_line` 早已指向下一条待执行指令，此时再加一会越过它，故退化为空操作
    /// （仅复位标记）。
    pub fn advance_line(&mut self) {
        if self.last_wait_from_queue {
            self.last_wait_from_queue = false;
            return;
        }
        self.current_line = self.current_line.saturating_add(1);
    }

    /// 获取变量存储的快照（用于存档）
    ///
    /// 变量现由 `Arc<Mutex<_>>` 持有，返回克隆快照以避免暴露锁。
    pub fn variables(&self) -> VariableStore {
        self.variables.lock().unwrap().clone()
    }

    /// 获取共享变量存储句柄（可变访问请锁定后操作）
    pub fn variables_handle(&self) -> Arc<Mutex<VariableStore>> {
        Arc::clone(&self.variables)
    }

    /// 获取解释器配置（包含从 system.ini 读取的环境变量）
    pub fn config(&self) -> &InterpreterConfig {
        &self.config
    }

    /// 便捷入口：加载指定脚本文件并从 `*main`/`*start`/文件开头开始执行
    ///
    /// 使用方解析 system.ini 后，直接将 BOOT 对应的脚本路径传入此方法即可。
    pub fn boot(&mut self, script: &str) -> Result<()> {
        self.load_external_script(script)?;

        let script_obj = self.scripts.get(script).unwrap();
        let start_label = if script_obj.get_label_line("main").is_some() {
            "main"
        } else if script_obj.get_label_line("start").is_some() {
            "start"
        } else if script_obj.get_label_line("_start").is_some() {
            "_start"
        } else {
            // 从文件开头
            self.current_script = Some(script.to_string());
            self.current_line = 0;
            self.call_stack.clear();
            return Ok(());
        };

        self.start(script, start_label)
    }

    /// 恢复变量状态（用于读档）
    pub fn restore_variables(&mut self, store: VariableStore) {
        *self.variables.lock().unwrap() = store;
    }

    /// 加载存档后恢复解释器执行位置。
    /// `jump_target` 为脚本内标签名（如 `"*title"`），解释器会从该标签继续执行。
    pub fn restore_position(
        &mut self,
        script: &str,
        line: usize,
        stack: Vec<CallFrame>,
    ) -> Result<()> {
        self.current_script = Some(script.to_string());
        self.current_line = line;
        self.call_stack = stack;
        // 重新加载目标脚本并定位到当前行
        self.load_external_script(script)?;
        Ok(())
    }

    /// 获取 Lua 上下文
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// 获取 Lua 上下文的可变引用
    pub fn lua_mut(&mut self) -> &mut Lua {
        &mut self.lua
    }

    /// 注册自定义标签处理器
    pub fn register_tag<H: crate::tags::TagHandler + 'static>(&mut self, name: &str, handler: H) {
        self.tag_registry.register(name, handler);
    }

    /// 设置事件回调
    pub fn set_callback<F: FnMut(Event) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) {
        self.callback = Box::new(callback);
    }

    /// 获取当前脚本
    pub fn current_script(&self) -> Option<&str> {
        self.current_script.as_deref()
    }

    /// 获取当前行号
    pub fn current_line(&self) -> usize {
        self.current_line
    }

    /// 获取调用栈快照（供存档序列化）
    pub fn call_stack(&self) -> Vec<CallFrame> {
        self.call_stack.clone()
    }

    /// 获取脚本
    pub fn get_script(&self, name: &str) -> Option<&Script> {
        self.scripts.get(name)
    }

    /// 设置变量
    pub fn set_variable(&mut self, name: &str, value: Value) {
        self.variables.lock().unwrap().set(name, value);
    }

    /// 获取变量（返回克隆值）
    pub fn get_variable(&self, name: &str) -> Option<Value> {
        self.variables.lock().unwrap().get(name).cloned()
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new(InterpreterConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::Interpreter;
    use crate::InterpreterConfig;

    #[test]
    fn pluto_preserves_mixed_numeric_and_string_table_keys() {
        let interpreter = Interpreter::new(InterpreterConfig::default());
        interpreter
            .lua()
            .load(
                r#"
                local saveslot = {
                    [1] = { file = "save0001" },
                    [4] = { file = "save0004" },
                    last = 4,
                    count = 4,
                    check = { save0001 = true, save0004 = true },
                }
                local encoded = pluto.persist({}, saveslot)
                restored_saveslot = pluto.unpersist({}, encoded)
                "#,
            )
            .exec()
            .unwrap();

        let globals = interpreter.lua().globals();
        let restored: mlua::Table = globals.get("restored_saveslot").unwrap();
        let slot1: mlua::Table = restored.get(1).unwrap();
        let slot4: mlua::Table = restored.get(4).unwrap();
        let check: mlua::Table = restored.get("check").unwrap();

        assert_eq!(slot1.get::<String>("file").unwrap(), "save0001");
        assert_eq!(slot4.get::<String>("file").unwrap(), "save0004");
        assert_eq!(restored.get::<i64>("last").unwrap(), 4);
        assert_eq!(restored.get::<i64>("count").unwrap(), 4);
        assert!(check.get::<bool>("save0001").unwrap());
        assert!(check.get::<bool>("save0004").unwrap());
    }
}
