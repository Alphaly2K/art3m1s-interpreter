# ASB Interpreter

ASB (Artemis Engine) 脚本解释器库。

## 特性

- ✅ ASB 二进制脚本解码（通过 `asb-decrypt` 库）
- ✅ 标签系统和参数解析
- ✅ 变量系统（普通变量、全局变量、临时变量、系统变量）
- ✅ 表达式求值（算术、逻辑、比较运算）
- ✅ Lua 脚本集成（通过 `mlua`）
- ✅ 可扩展的标签处理器系统
- ✅ 事件回调机制
- ✅ 存档/读档支持
- ✅ 图层系统（支持图层集和属性批量设置）
- ✅ 宏系统（将标签组合为新标签）
- ✅ 条件标签（if/else/endif）

## 安装

```toml
[dependencies]
asb-interpreter = { path = "path/to/asb-interpreter" }
```

## 快速开始

```rust
use asb_interpreter::{Interpreter, InterpreterConfig, Event, CallbackResult, ExecutionResult};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建解释器
    let mut interpreter = Interpreter::new(InterpreterConfig::default());

    // 加载脚本（从文本）
    let script = r#"
*main
[var name="message" data="'Hello, World!'"]
[calllua function="showMessage"]
[return]
"#;
    interpreter.load_script("test", script)?;

    // 设置事件回调
    interpreter.set_callback(|event| {
        match event {
            Event::Text { content } => {
                println!("文本: {}", content);
                CallbackResult::Continue
            }
            Event::UiTransition(trans) => {
                println!("转场效果: {}ms", trans.time);
                CallbackResult::Continue
            }
            _ => CallbackResult::Continue,
        }
    });

    // 执行脚本
    interpreter.start("test", "main")?;
    
    loop {
        match interpreter.run()? {
            ExecutionResult::Completed => break,
            ExecutionResult::Wait(event) => {
                // 处理等待状态（显示UI、等待输入等）
                println!("等待: {:?}", event);
                // ... 处理完成后继续
            }
            _ => {}
        }
    }

    Ok(())
}
```

## 从 ASB 二进制文件加载

```rust
use asb_interpreter::{Interpreter, InterpreterConfig};

let mut interpreter = Interpreter::new(InterpreterConfig::default());

// 从 ASB 二进制数据加载
let asb_data = std::fs::read("script.asb")?;
interpreter.load_asb("system/script", &asb_data)?;

interpreter.start("system/script", "main")?;
interpreter.run()?;
```

## 使用脚本加载器

```rust
use asb_interpreter::{Interpreter, InterpreterConfig};

let mut interpreter = Interpreter::new(InterpreterConfig::default());

// 设置脚本加载器（用于跨脚本调用）
interpreter.set_script_loader(Box::new(|name| {
    let path = format!("game/scripts/{}.asb", name);
    let data = std::fs::read(&path)?;
    asb_decrypt::decode_asb_to_string(&data)
        .map_err(|e| e.into())
}));

// 解释器会自动在需要时加载外部脚本
```

## 存档/读档

```rust
use asb_interpreter::{Interpreter, VariableStore};

// 存档
fn save_game(interpreter: &Interpreter) -> Vec<u8> {
    interpreter.variables().save().unwrap()
}

// 读档
fn load_game(interpreter: &mut Interpreter, data: &[u8]) {
    let vars = VariableStore::load(data).unwrap();
    interpreter.restore_variables(vars);
}
```

## 自定义标签

```rust
use asb_interpreter::{TagHandler, ExecutionContext, TagResult, Event};

struct MyCustomTag;

impl TagHandler for MyCustomTag {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> asb_interpreter::Result<TagResult> {
        let value = ctx.resolve_param("data")?;
        println!("自定义标签数据: {}", value);
        
        Ok(TagResult::Continue)
    }
}

// 注册自定义标签
interpreter.register_tag("mytag", MyCustomTag);
```

## 变量系统

ASB 脚本支持四种变量类型：

- **普通变量**: `foo`, `bar` - 局部作用域
- **全局变量**: `g.score`, `g.flag` - 跨存档持久化
- **临时变量**: `t.temp`, `t.text` - 不写入存档
- **系统变量**: `s.config` - 系统配置相关

```rust
use asb_interpreter::{Interpreter, Value};

let mut interpreter = Interpreter::default();

// 设置变量
interpreter.set_variable("score", Value::Int(100));
interpreter.set_variable("g.global_flag", Value::Bool(true));
interpreter.set_variable("t.temp_text", Value::String("Hello".into()));

// 获取变量
if let Some(Value::Int(score)) = interpreter.get_variable("score") {
    println!("Score: {}", score);
}
```

## 表达式语法

支持以下运算符：

- 算术: `+`, `-`, `*`, `/`, `%`
- 比较: `==`, `!=`, `<`, `<=`, `>`, `>=`
- 逻辑: `&&`, `||`
- 字符串连接: `+`（当操作数为字符串时）
- 十六进制: `0xFF`
- 括号: `(1 + 2) * 3`

变量引用以 `$` 开头：

```
[var name="foo" data="10"]
[var name="bar" data="$foo + 5"]      # bar = 15
[var name="text" data="$foo + 'pts'"] # text = "10pts"
```

## Lua 集成

```rust
use mlua::Lua;

// 访问 Lua 上下文
let lua = interpreter.lua();

// 在 Lua 中设置函数
lua.load(r#"
    function showMessage()
        print("Hello from Lua!")
    end
    
    -- 嵌套命名空间
    sv = {}
    function sv.save()
        print("Saving game...")
    end
"#).exec()?;

// 脚本可以调用这些函数
// [calllua function="showMessage"]
// [calllua function="sv.save"]
```

## 支持的标签

### 控制流
- `[jump label="xxx"]` - 跳转
- `[jump file="xxx.asb" label="xxx"]` - 跨文件跳转
- `[jump cond="条件" label="xxx"]` - 条件跳转
- `[call label="xxx"]` - 调用
- `[call file="xxx.asb" label="xxx"]` - 跨文件调用
- `[return]` - 返回

### 等待
- `[stop]` - 停止
- `[wt]` - 等待
- `[wt0]` - 等待（变体）
- `[wait time="1000"]` - 时间等待
- `[exkey btn="xxx"]` - 按键等待

### 变量
- `[var name="xxx" data="value"]` - 设置变量

### Lua
- `[calllua function="xxx"]` - 调用 Lua 函数

### UI
- `[uitrans]` - UI 转场
- `[uitrans time="500"]` - 指定时间转场
- `[uitrans fade="xxx"]` - 淡入淡出
- `[loading 0="on"]` - 加载状态
- `[saving 0="on"]` - 存档状态
- `[sysshow]` / `[syshide]` - 系统 UI 显示/隐藏
- `[lyc id="xxx" file="xxx"]` - 创建图层
- `[lyc2 id="xxx" file="xxx"]` - 创建图层（变体）
- `[lydel id="xxx"]` - 删除图层
- `[lyprop id="xxx" visible="1"]` - 图层属性

### 存档
- `[syssave]` - 系统存档
- `[reset]` - 重置

### 其他
- `[dialog title="xxx" message="xxx"]` - 对话框
- `[yesno file="xxx"]` - 是/否选择
- `[exit]` - 退出
- `[gotitle]` - 返回标题
- `[@]` - 暂停等待

## 许可证

MIT

## 宏系统

宏允许你将多个标签组合成新的自定义标签。

### 定义宏

宏定义在脚本文件中（通常是 `macro.iet`）：

```
*chara_a
[if estimate="$pos == 'left'"]
    [lyc id="1" file="chara_a"]
[/if]
[if estimate="$pos == 'center'"]
    [lyc id="3" file="chara_a"]
    [lyprop id="3" left="120"]
[/if]
[if estimate="$pos == 'right'"]
    [lyc id="2" file="chara_a"]
    [lyprop id="2" left="240"]
[/if]
[trans type="1" time="1000"]
[return]
```

### 使用宏

```rust
use asb_interpreter::{MacroRegistry, Script};

// 加载宏文件
let macro_script = Script::parse("macro", &macro_content)?;
let mut registry = MacroRegistry::new();
registry.load_from_script(&macro_script)?;

// 在解释器中使用
// [chara_a pos="center"]
```

### 参数传递

宏的参数会自动展开为变量：

```
*show_text
[print data="$param0"]
[print data="$param1"]
[return]
```

调用 `[show_text param0="Hello" param1="World"]` 会显示 Hello 和 World。

### 检查参数

使用 `var system="var_exist"` 检查参数是否传递：

```
*new_tag
[var system="var_exist" name="t.result" target="param0" local="1"]
[if estimate="$t.result"]
    [print data="$param0"]
[else]
    [print data="param0 未传递"]
[/if]
[return]
```

## 图层系统

### 图层ID

- 纯数字ID：按数字大小排序（0, 1, 2, ...）
- 包含非数字：按字符串排序
- 建议使用纯数字以避免问题

### 图层集

使用点号分隔创建图层集：

```
[lyprop id="1.0" left="10" top="10"]
[lyprop id="1.1" left="10" top="10"]

; 或者批量设置图层集
[lyprop id="1" left="10" top="10"]
```

### lyprop 属性

支持的属性：
- `left` / `top` - 位置
- `width` / `height` - 尺寸
- `alpha` - 透明度 (0-255)
- `visible` - 可见性
- `scale_x` / `scale_y` - 缩放
- `rotation` - 旋转

## 条件标签

### if/else/endif

```
[if estimate="$score >= 100"]
    [print data="高分！"]
[else]
    [print data="继续加油"]
[/if]
```

### 嵌套条件

```
[if estimate="$a == 1"]
    [if estimate="$b == 2"]
        [print data="a=1 且 b=2"]
    [/if]
[/if]
```

## 许可证

MIT
