# ASB 解释器库 API 设计

## 项目概述
构建一个 ASB (Artemis Engine) 脚本解释器库，支持游戏引擎集成，使用 mlua 实现 Lua 脚本调用。

## 架构设计

### 核心模块

```
asb-interpreter/
├── src/
│   ├── lib.rs              # 公共API导出
│   ├── interpreter.rs      # 主解释器
│   ├── script.rs           # 脚本解析与表示
│   ├── variable.rs         # 变量存储系统
│   ├── expression.rs       # 表达式求值器
│   ├── lua_ctx.rs          # Lua 上下文管理
│   ├── tags/               # 标签处理器
│   │   ├── mod.rs
│   │   ├── control.rs      # 控制流标签 (jump, call, return, stop)
│   │   ├── lua.rs          # calllua 标签
│   │   ├── ui.rs           # UI 相关标签 (uitrans, lyc, lydel 等)
│   │   ├── save.rs         # 存档相关标签
│   │   └── custom.rs       # 自定义标签注册
│   ├── event.rs            # 事件系统
│   └── error.rs            # 错误类型定义
```

### 核心类型设计

#### 1. Interpreter - 主解释器

```rust
/// ASB 脚本解释器
pub struct Interpreter {
    // 内部状态
}

impl Interpreter {
    /// 创建新的解释器实例
    pub fn new(config: InterpreterConfig) -> Self;
    
    /// 加载脚本文件（从内存）
    pub fn load_script(&mut self, name: &str, content: &str) -> Result<()>;
    
    /// 加载脚本文件（通过回调）
    pub fn set_script_loader(&mut self, loader: ScriptLoader);
    
    /// 设置起始标签
    pub fn start(&mut self, label: &str) -> Result<()>;
    
    /// 执行到下一个等待点（stop/wt/exkey等）
    pub fn step(&mut self) -> Result<ExecutionResult>;
    
    /// 持续执行直到完成或等待
    pub fn run(&mut self) -> Result<ExecutionResult>;
    
    /// 获取变量存储（用于存档）
    pub fn variables(&self) -> &VariableStore;
    
    /// 恢复变量状态（用于读档）
    pub fn restore_variables(&mut self, store: VariableStore);
    
    /// 获取 Lua 上下文（用于扩展）
    pub fn lua(&self) -> &mlua::Lua;
    
    /// 注册自定义标签处理器
    pub fn register_tag<T: TagHandler>(&mut self, name: &str, handler: T);
    
    /// 设置事件回调
    pub fn set_callback<F: FnMut(Event) -> CallbackResult>(&mut self, callback: F);
}
```

#### 2. InterpreterConfig - 配置

```rust
pub struct InterpreterConfig {
    /// 脚本编码（默认 Shift_JIS）
    pub encoding: Encoding,
    /// 是否启用行标签自动插入
    pub auto_insert_labels: bool,
}

impl Default for InterpreterConfig { ... }
```

#### 3. Script - 脚本表示

```rust
/// 解析后的脚本
pub struct Script {
    name: String,
    labels: HashMap<String, usize>,  // 标签名 -> 行号
    instructions: Vec<Instruction>,   // 指令序列
}

/// 单条指令
pub struct Instruction {
    pub tag: String,
    pub params: HashMap<String, String>,
    pub line: usize,
}
```

#### 4. VariableStore - 变量存储

```rust
/// 变量存储（支持序列化）
#[derive(Serialize, Deserialize)]
pub struct VariableStore {
    /// 普通变量
    local: HashMap<String, Value>,
    /// 全局变量 (g.)
    global: HashMap<String, Value>,
    /// 临时变量 (t.)
    temp: HashMap<String, Value>,
    /// 系统变量 (s.)
    system: HashMap<String, Value>,
}

/// 变量值
#[derive(Clone, Serialize, Deserialize)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

impl VariableStore {
    pub fn get(&self, name: &str) -> Option<&Value>;
    pub fn set(&mut self, name: &str, value: Value);
    pub fn clear_temp(&mut self);  // 清除临时变量
    pub fn save(&self) -> Vec<u8>; // 序列化
    pub fn load(data: &[u8]) -> Result<Self>; // 反序列化
}
```

#### 5. Expression - 表达式求值

```rust
/// 表达式求值器
pub struct ExpressionEvaluator<'a> {
    variables: &'a VariableStore,
    lua: &'a mlua::Lua,
}

impl<'a> ExpressionEvaluator<'a> {
    /// 求值表达式（以 $ 开头）
    pub fn evaluate(&self, expr: &str) -> Result<Value>;
    
    /// 解析参数值（处理表达式和字面量）
    pub fn resolve_param(&self, value: &str) -> Result<Value>;
}
```

#### 6. ExecutionResult - 执行结果

```rust
/// 执行结果
pub enum ExecutionResult {
    /// 执行完成
    Completed,
    /// 等待用户输入
    Wait(WaitReason),
    /// 调用外部脚本
    CallScript { file: String, label: String },
    /// 跳转到其他脚本
    JumpScript { file: String, label: String },
}

/// 等待原因
pub enum WaitReason {
    /// 通用等待 [wt]
    Generic,
    /// 停止 [stop]
    Stop { reason: Option<String> },
    /// 按键等待 [exkey]
    KeyWait { buttons: Vec<String> },
    /// 时间等待 [wait time="xxx"]
    Timed { milliseconds: u64 },
    /// 是/否选择 [yesno]
    YesNo { file: String },
}
```

#### 7. Event - 事件系统

```rust
/// 解释器事件
pub enum Event {
    /// UI 转场
    UiTransition { params: TransitionParams },
    /// 图层操作
    LayerOperation(LayerOp),
    /// 加载状态变化
    LoadingStateChanged { active: bool },
    /// 存档状态变化
    SavingStateChanged { active: bool },
    /// 音效播放
    PlaySound { name: String, wait: bool },
    /// 对话框显示
    ShowDialog { title: String, message: String },
    /// 脚本调用
    ScriptCall { file: String, label: String },
    /// 退出请求
    Exit,
    /// 返回标题
    GoTitle,
    /// 自定义事件
    Custom { tag: String, params: HashMap<String, String> },
}

/// 回调结果
pub enum CallbackResult {
    /// 继续执行
    Continue,
    /// 暂停执行（等待外部事件）
    Pause,
    /// 中止执行
    Abort,
}
```

#### 8. TagHandler - 标签处理器

```rust
/// 标签处理器 trait
pub trait TagHandler: Send + Sync {
    /// 执行标签
    fn execute(
        &self,
        ctx: &mut ExecutionContext,
        params: &HashMap<String, String>,
    ) -> Result<TagResult>;
}

/// 标签执行结果
pub enum TagResult {
    /// 继续下一条
    Continue,
    /// 跳转
    Jump(usize),
    /// 等待
    Wait(WaitReason),
    /// 事件
    Event(Event),
}

/// 执行上下文
pub struct ExecutionContext<'a> {
    pub interpreter: &'a mut Interpreter,
    pub variables: &'a mut VariableStore,
    pub lua: &'a mlua::Lua,
    pub current_script: &'a str,
    pub current_line: usize,
}
```

#### 9. ScriptLoader - 脚本加载器

```rust
/// 脚本加载器类型
pub type ScriptLoader = Box<dyn Fn(&str) -> Result<String> + Send + Sync>;

// 使用示例：
interpreter.set_script_loader(Box::new(|name| {
    let path = format!("game/scripts/{}.asb", name);
    let data = std::fs::read(&path)?;
    asb_decrypt::decode_asb_to_string(&data)
        .map_err(|e| e.into())
}));
```

### Lua 集成

```rust
// 在 Lua 上下文中自动注入的变量和函数
// Lua 脚本可以通过这些与解释器交互

// 变量访问
fn get_var(name: &str) -> Value;
fn set_var(name: &str, value: Value);

// 解释器控制
fn jump(label: &str);
fn call(file: &str, label: &str);
fn stop();
fn wait(ms: u64);

// 回调注册
fn register_callback(event: &str, func: Function);
```

### 错误处理

```rust
#[derive(Debug)]
pub enum Error {
    /// 脚本解析错误
    ParseError { line: usize, message: String },
    /// 运行时错误
    RuntimeError { line: usize, message: String },
    /// 表达式求值错误
    ExpressionError(String),
    /// 变量未定义
    UndefinedVariable(String),
    /// 标签未找到
    LabelNotFound(String),
    /// 脚本未找到
    ScriptNotFound(String),
    /// Lua 错误
    LuaError(mlua::Error),
    /// IO 错误
    IoError(std::io::Error),
    /// 序列化错误
    SerializeError(String),
}
```

## 使用示例

### 基本使用

```rust
use asb_interpreter::{Interpreter, InterpreterConfig, Event, CallbackResult};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建解释器
    let mut interpreter = Interpreter::new(InterpreterConfig::default());
    
    // 设置脚本加载器
    interpreter.set_script_loader(Box::new(|name| {
        let path = format!("game/{}.asb", name);
        let data = std::fs::read(&path)?;
        asb_decrypt::decode_asb_to_string(&data)
            .map_err(|e| e.into())
    }));
    
    // 设置事件回调
    interpreter.set_callback(|event| {
        match event {
            Event::UiTransition { .. } => {
                // 播放转场动画
                CallbackResult::Continue
            }
            Event::ShowDialog { title, message } => {
                // 显示对话框
                CallbackResult::Pause  // 等待用户关闭
            }
            _ => CallbackResult::Continue,
        }
    });
    
    // 加载并执行脚本
    interpreter.load_script("system/script", &script_content)?;
    interpreter.start("main")?;
    
    // 游戏循环
    loop {
        match interpreter.run()? {
            ExecutionResult::Completed => break,
            ExecutionResult::Wait(reason) => {
                // 处理等待状态（显示UI、等待输入等）
                handle_wait(reason);
            }
            ExecutionResult::CallScript { file, label } => {
                // 加载并调用其他脚本
                interpreter.load_script(&file, &load_script(&file)?)?;
                interpreter.start(&label)?;
            }
            _ => {}
        }
    }
    
    Ok(())
}
```

### 存档/读档

```rust
// 存档
fn save_game(interpreter: &Interpreter) -> Vec<u8> {
    interpreter.variables().save()
}

// 读档
fn load_game(interpreter: &mut Interpreter, data: &[u8]) {
    let vars = VariableStore::load(data).unwrap();
    interpreter.restore_variables(vars);
}
```

### 自定义标签

```rust
use asb_interpreter::{TagHandler, ExecutionContext, TagResult};

struct MyCustomTag;

impl TagHandler for MyCustomTag {
    fn execute(
        &self,
        ctx: &mut ExecutionContext,
        params: &HashMap<String, String>,
    ) -> Result<TagResult> {
        // 自定义逻辑
        let value = params.get("data").unwrap();
        println!("Custom tag: {}", value);
        
        Ok(TagResult::Continue)
    }
}

// 注册
interpreter.register_tag("mytag", MyCustomTag);
```

## 依赖

```toml
[dependencies]
asb-decrypt = { path = "../asb-decrypt" }
mlua = { version = "0.9", features = ["lua54", "send"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
encoding_rs = "0.8"  # 支持 Shift_JIS 编码
thiserror = "1.0"
```

## 实现优先级

### Phase 1: 核心基础
- [ ] 脚本解析器（标签、参数、变量）
- [ ] 变量存储系统
- [ ] 表达式求值器
- [ ] 基础控制流（jump, return, stop）
- [ ] 错误处理

### Phase 2: Lua 集成
- [ ] mlua 上下文初始化
- [ ] calllua 标签实现
- [ ] Lua <-> Rust 变量桥接
- [ ] Lua 函数注册机制

### Phase 3: 游戏引擎集成
- [ ] 事件系统
- [ ] 回调机制
- [ ] 脚本加载器
- [ ] 跨脚本调用

### Phase 4: 高级特性
- [ ] 存档/读档序列化
- [ ] 条件跳转
- [ ] UI 标签处理
- [ ] 自定义标签注册

## 设计决策

1. **为什么用回调而不是直接实现 UI？**
   - 保持库的通用性，不绑定特定渲染后端
   - 让用户根据实际需求实现 UI 逻辑

2. **为什么变量存储支持序列化？**
   - 游戏存档需要持久化变量状态
   - 使用 serde 实现，支持多种格式

3. **为什么用 trait 定义 TagHandler？**
   - 允许用户扩展自定义标签
   - 便于测试和维护

4. **为什么 Lua 是可选的？**
   - 不是所有脚本都需要 Lua
   - 减少不必要的依赖
