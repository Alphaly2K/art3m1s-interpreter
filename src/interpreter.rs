//! 主解释器（迭代版本）
//!
//! ASB 脚本解释器的核心实现，使用迭代而非递归来避免栈溢出。

use crate::error::{Error, Result};
use crate::event::{CallbackResult, Event, EventCallback, ScriptLoader, default_callback};
use crate::lua_engine::{DefaultEngineCallbacks, EngineContext};
use crate::script::{Instruction, Script};
use crate::tags::{
    ExecutionContext, TagRegistry, TagResult,
};
use crate::variable::{Value, VariableStore};
use mlua::Lua;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 调用栈帧
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// 脚本名
    pub script: String,
    /// 返回行号
    pub return_line: usize,
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
}

impl Interpreter {
    /// 创建新的解释器实例
    pub fn new(config: InterpreterConfig) -> Self {
        let lua = Lua::new();
        let variables = Arc::new(Mutex::new(VariableStore::new()));
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
        let script_obj = self.scripts.get(script).ok_or_else(|| {
            Error::ScriptNotFound(script.to_string())
        })?;

        let line = script_obj.get_label_line(label).ok_or_else(|| {
            Error::LabelNotFound(label.to_string())
        })?;

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
                        return Ok(ExecutionResult::Wait(Event::ScenarioText { content: text, inline: false }));
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
                TagResult::Call { file, label, return_line, return_script } => {
                    // 压入调用栈
                    self.call_stack.push(CallFrame {
                        script: return_script.clone(),
                        return_line,
                    });

                    // 跳转到目标
                    if let Some(target_file) = file {
                        // 跨脚本调用
                        self.load_external_script(&target_file)?;
                        let target_script = self.scripts.get(&target_file)
                            .ok_or_else(|| Error::ScriptNotFound(target_file.clone()))?;
                        let target_line = target_script.get_label_line(&label)
                            .ok_or_else(|| Error::LabelNotFound(label.clone()))?;

                        self.current_script = Some(target_file.clone());
                        self.current_line = target_line;
                        continue;
                    } else {
                        // 同脚本调用
                        let script = self.scripts.get(&return_script).unwrap();
                        let line = script.get_label_line(&label)
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
                    let result = (self.callback)(event);
                    match result {
                        CallbackResult::Continue => {
                            self.current_line += 1;
                            continue;
                        }
                        CallbackResult::Pause => {
                            return Ok(ExecutionResult::Completed);
                        }
                        CallbackResult::Abort => {
                            return Err(Error::Aborted);
                        }
                    }
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
                                    return Ok(ExecutionResult::Completed);
                                }
                                CallbackResult::Abort => {
                                    return Err(Error::Aborted);
                                }
                            }
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
                                TagResult::Call { file, label, return_line, return_script } => {
                                    self.call_stack.push(CallFrame {
                                        script: return_script,
                                        return_line,
                                    });
                                    if let Some(target_file) = file {
                                        self.load_external_script(&target_file)?;
                                        let target_script = self.scripts.get(&target_file)
                                            .ok_or_else(|| Error::ScriptNotFound(target_file.clone()))?;
                                        let target_line = target_script.get_label_line(&label)
                                            .ok_or_else(|| Error::LabelNotFound(label.clone()))?;
                                        self.current_script = Some(target_file);
                                        self.current_line = target_line;
                                    } else {
                                        let script_name = self.current_script.clone().unwrap_or_default();
                                        let script = self.scripts.get(&script_name).unwrap();
                                        let line = script.get_label_line(&label)
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
                        CallbackResult::Pause => return Ok(Some(ExecutionResult::Wait(event))),
                        CallbackResult::Abort => return Err(Error::Aborted),
                    }
                }
                // Lua 也会通过 eqtag/enqueueTag 排入控制流标签，最典型的是
                // `eqtag{"jump", file=..., label="game_start"}`（跨脚本跳转返回
                // TagResult::Call）。必须落实位置变更，否则 boot 推进不到 game_start。
                // 改动 current_script/current_line 后继续抽干队列；flush 返回后
                // step 主循环会从新位置读取指令。
                TagResult::Jump(line) => {
                    self.current_line = line;
                    continue;
                }
                TagResult::Call { file, label, return_line, return_script } => {
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
                    continue;
                }
                TagResult::Return => {
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
            // 未注册的标签，发出自定义事件
            Ok(TagResult::Emit(Event::Custom {
                tag: instruction.tag.clone(),
                params: instruction.params.clone(),
            }))
        }
    }

    /// 持续执行直到完成或等待
    pub fn run(&mut self) -> Result<ExecutionResult> {
        self.step()
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
    pub fn set_callback<F: FnMut(Event) -> CallbackResult + Send + Sync + 'static>(&mut self, callback: F) {
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
