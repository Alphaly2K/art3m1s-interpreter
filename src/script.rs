//! 脚本解析与表示
//!
//! 解析 ASB 脚本文本，生成可执行的指令序列。

use crate::error::{Error, Result};
use std::collections::HashMap;

/// 单条指令
#[derive(Debug, Clone)]
pub struct Instruction {
    /// 标签名
    pub tag: String,
    /// 参数键值对
    pub params: HashMap<String, String>,
    /// 原始行号
    pub line: usize,
}

impl Instruction {
    /// 获取参数值
    pub fn get(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }

    /// 获取参数值，如果不存在则返回默认值
    pub fn get_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.params.get(key).map(|s| s.as_str()).unwrap_or(default)
    }

    /// 获取无键参数（第一个参数或 key 为 "0" 的参数）
    pub fn get_default(&self) -> Option<&str> {
        self.params.get("0").or(self.params.values().next()).map(|s| s.as_str())
    }

    /// 检查是否有某个参数
    pub fn has(&self, key: &str) -> bool {
        self.params.contains_key(key)
    }
}

/// 解析后的脚本
#[derive(Debug, Clone)]
pub struct Script {
    /// 脚本名称
    pub name: String,
    /// 标签名到行号的映射
    pub labels: HashMap<String, usize>,
    /// 指令序列
    pub instructions: Vec<Instruction>,
}

impl Script {
    /// 从文本解析脚本
    pub fn parse(name: &str, content: &str) -> Result<Self> {
        let mut labels = HashMap::new();
        let mut instructions = Vec::new();

        let lines: Vec<&str> = content.lines().collect();
        let mut line_idx = 0;

        'lines: while line_idx < lines.len() {
            let line = lines[line_idx].trim();
            let line_num = line_idx + 1;

            // 跳过空行
            if line.is_empty() {
                line_idx += 1;
                continue;
            }

            // 标签定义 (*labelname)
            if let Some(label_name) = line.strip_prefix('*') {
                let label_name = label_name.trim().to_string();
                if !label_name.is_empty() {
                    labels.insert(label_name, instructions.len());
                }
                line_idx += 1;
                continue;
            }

            // 指令解析 - 一行中可能有多个标签
            if line.contains('[') {
                let tags = extract_tags_from_line(line);

                for tag_str in tags {
                    let inner = &tag_str[1..tag_str.len() - 1];

                    // 检查是否是 [lua] 块开始
                    if inner.trim() == "lua" {
                        // 收集直到 [/lua] 的所有行作为 Lua 代码
                        let mut lua_code = String::new();
                        line_idx += 1;
                        let mut found_end = false;

                        while line_idx < lines.len() {
                            let lua_line = lines[line_idx].trim();
                            if lua_line == "[/lua]" {
                                found_end = true;
                                line_idx += 1;
                                break;
                            }
                            if !lua_code.is_empty() {
                                lua_code.push('\n');
                            }
                            lua_code.push_str(lines[line_idx]);
                            line_idx += 1;
                        }

                        if !found_end {
                            return Err(Error::ParseError {
                                line: line_num,
                                message: "未找到 [/lua] 结束标记".to_string(),
                            });
                        }

                        // 创建特殊的 Lua 块指令
                        let mut params = HashMap::new();
                        params.insert("code".to_string(), lua_code);
                        instructions.push(Instruction {
                            tag: "__lua_block".to_string(),
                            params,
                            line: line_num,
                        });
                        // line_idx 已指向 [/lua] 的下一行，直接进入外层循环下一轮，
                        // 不能再走末尾的统一 +1，否则会吞掉 [/lua] 紧邻的下一行。
                        continue 'lines;
                    }

                    match parse_instruction(inner, line_num) {
                        Ok(instruction) => {
                            instructions.push(instruction);
                        }
                        Err(e) => {
                            return Err(Error::ParseError {
                                line: line_num,
                                message: e.to_string(),
                            });
                        }
                    }
                }
                line_idx += 1;
                continue;
            }

            // 剧情文本（未被 [] 包围的内容）
            // 创建特殊的 text 标签
            let mut params = HashMap::new();
            params.insert("text".to_string(), line.to_string());
            instructions.push(Instruction {
                tag: "__text".to_string(),
                params,
                line: line_num,
            });
            line_idx += 1;
        }

        Ok(Script {
            name: name.to_string(),
            labels,
            instructions,
        })
    }

    /// 获取标签对应的行号
    pub fn get_label_line(&self, label: &str) -> Option<usize> {
        self.labels.get(label).copied()
    }

    /// 获取指令
    pub fn get_instruction(&self, line: usize) -> Option<&Instruction> {
        self.instructions.get(line)
    }

    /// 获取指令数量
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}

/// 从一行文本中提取所有标签（支持一行多个标签）
///
/// 例如: "[if ...][load ...][/if]" -> ["[if ...]", "[load ...]", "[/if]"]
fn extract_tags_from_line(line: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut current = String::new();
    let mut in_tag = false;

    for ch in line.chars() {
        match ch {
            '[' => {
                if in_tag {
                    // 嵌套的 '['，忽略
                    current.push(ch);
                } else {
                    in_tag = true;
                    current.clear();
                    current.push(ch);
                }
            }
            ']' => {
                if in_tag {
                    current.push(ch);
                    tags.push(current.clone());
                    current.clear();
                    in_tag = false;
                }
            }
            _ => {
                if in_tag {
                    current.push(ch);
                }
            }
        }
    }

    tags
}

/// 解析指令内容
fn parse_instruction(content: &str, line: usize) -> Result<Instruction> {
    let content = content.trim();

    if content.is_empty() {
        return Err(Error::ParseError {
            line,
            message: "空指令".to_string(),
        });
    }

    // 分割标签名和参数
    let mut parts = content.splitn(2, char::is_whitespace);
    let tag = parts.next().unwrap().trim().to_string();
    let params_str = parts.next().unwrap_or("").trim();

    let params = parse_params(params_str, line)?;

    Ok(Instruction { tag, params, line })
}

/// 解析参数字符串
fn parse_params(params_str: &str, line: usize) -> Result<HashMap<String, String>> {
    let mut params = HashMap::new();
    let mut chars = params_str.chars().peekable();
    let mut param_index = 0;

    while chars.peek().is_some() {
        // 跳过空白
        while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
            chars.next();
        }

        if chars.peek().is_none() {
            break;
        }

        // 读取键
        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c.is_whitespace() {
                break;
            }
            key.push(chars.next().unwrap());
        }

        if key.is_empty() {
            break;
        }

        // 跳过空白
        while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
            chars.next();
        }

        // 检查是否有 =
        if chars.peek() != Some(&'=') {
            // 无值参数，使用索引作为键
            params.insert(param_index.to_string(), key);
            param_index += 1;
            continue;
        }

        // 消耗 =
        chars.next();

        // 跳过空白
        while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
            chars.next();
        }

        // 读取值
        let value = if chars.peek() == Some(&'"') {
            // 引号包裹的值
            chars.next(); // 消耗开头的引号
            let mut value = String::new();
            loop {
                match chars.next() {
                    Some('"') => break,
                    Some(c) => value.push(c),
                    None => {
                        return Err(Error::ParseError {
                            line,
                            message: "未闭合的引号".to_string(),
                        });
                    }
                }
            }
            value
        } else {
            // 无引号的值
            let mut value = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                value.push(chars.next().unwrap());
            }
            value
        };

        // 如果键是数字，转换为索引格式
        if key.chars().all(|c| c.is_ascii_digit()) {
            params.insert(key, value);
        } else {
            params.insert(key, value);
        }

        param_index += 1;
    }

    Ok(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_script() {
        let content = r#"
*main
[calllua function="scriptMainloop"]
[jump label="next"]
*next
[return]
"#;

        let script = Script::parse("test", content).unwrap();
        assert_eq!(script.labels.get("main"), Some(&0));
        assert_eq!(script.labels.get("next"), Some(&2));
        assert_eq!(script.instructions.len(), 3);
    }

    #[test]
    fn test_parse_params() {
        let content = r#"
*test
[uitrans 0="500" time="1000"]
[uitrans fade="$t.trns"]
[jump cond="$t.check==0" label="next"]
"#;

        let script = Script::parse("test", content).unwrap();

        let inst1 = &script.instructions[0];
        assert_eq!(inst1.tag, "uitrans");
        assert_eq!(inst1.get("0"), Some("500"));
        assert_eq!(inst1.get("time"), Some("1000"));

        let inst2 = &script.instructions[1];
        assert_eq!(inst2.get("fade"), Some("$t.trns"));

        let inst3 = &script.instructions[2];
        assert_eq!(inst3.get("cond"), Some("$t.check==0"));
        assert_eq!(inst3.get("label"), Some("next"));
    }

    #[test]
    fn test_parse_text() {
        let content = r#"
*main
这是剧情文本
[return]
"#;

        let script = Script::parse("test", content).unwrap();
        assert_eq!(script.instructions[0].tag, "__text");
        assert_eq!(
            script.instructions[0].get("text"),
            Some("这是剧情文本")
        );
    }

    #[test]
    fn test_instruction_methods() {
        let inst = Instruction {
            tag: "test".to_string(),
            params: {
                let mut m = HashMap::new();
                m.insert("key1".to_string(), "value1".to_string());
                m.insert("0".to_string(), "default".to_string());
                m
            },
            line: 1,
        };

        assert_eq!(inst.get("key1"), Some("value1"));
        assert_eq!(inst.get("key2"), None);
        assert_eq!(inst.get_or("key2", "fallback"), "fallback");
        assert_eq!(inst.get_default(), Some("default"));
        assert!(inst.has("key1"));
        assert!(!inst.has("key2"));
    }
}
