//! var 标签的 system= 变体处理器
//!
//! 实现 [var system="xxx"] 的各种系统功能。

use crate::error::Result;
use crate::variable::{Value, VariableStore};
use std::collections::HashMap;

/// 执行 var system= 变体
pub fn execute_var_system(
    system: &str,
    params: &HashMap<String, String>,
    resolved: &HashMap<String, Value>,
    variables: &mut VariableStore,
) -> Result<()> {
    // 辅助函数：获取已解析的参数值
    let get_resolved = |key: &str| -> Option<&Value> {
        resolved.get(key)
    };

    match system {
        "var_exist" => {
            let target = params.get("target").map(|s| s.as_str()).unwrap_or("");
            let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
            let local = params.get("local").map(|s| s.as_str()) == Some("1");

            let exists = if local {
                variables.get(target).is_some()
                    && !target.starts_with("g.")
                    && !target.starts_with("t.")
                    && !target.starts_with("s.")
            } else {
                variables.contains(target)
            };

            variables.set(name, Value::Bool(exists));
        }

        "delete" => {
            if let Some(name) = params.get("name") {
                if name.is_empty() {
                    variables.clear_all();
                } else {
                    delete_variable_tree(variables, name);
                }
            } else {
                variables.clear_all();
            }
        }

        "random" => {
            let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
            let min = get_resolved("min").and_then(|v| v.as_int()).unwrap_or(0);
            let max = get_resolved("max").and_then(|v| v.as_int()).unwrap_or(i64::MAX);

            let result = if max > min {
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as i64)
                    .unwrap_or(0);
                let range = (max - min + 1) as u64;
                let rand_val = ((seed.wrapping_mul(6364136223846793005).wrapping_add(1)) as u64) % range;
                min + rand_val as i64
            } else {
                min
            };
            variables.set(name, Value::Int(result));
        }

        "length" => {
            let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
            let source = get_resolved("source").map(|v| v.as_string()).unwrap_or_default();
            let mode = params.get("mode").map(|s| s.as_str()).unwrap_or("0");

            let len = if mode == "1" {
                source.chars().count() as i64
            } else {
                source.len() as i64
            };
            variables.set(name, Value::Int(len));
        }

        "find" => {
            let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
            let source = get_resolved("source").map(|v| v.as_string()).unwrap_or_default();
            let string = get_resolved("string").map(|v| v.as_string()).unwrap_or_default();

            let pos = source.find(&string).map(|p| p as i64).unwrap_or(-1);
            variables.set(name, Value::Int(pos));
        }

        "substr" => {
            let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
            let source = get_resolved("source").map(|v| v.as_string()).unwrap_or_default();
            let position = get_resolved("position").and_then(|v| v.as_int()).unwrap_or(0) as usize;
            let length = get_resolved("length")
                .and_then(|v| v.as_int())
                .unwrap_or(source.len() as i64) as usize;
            let mode = params.get("mode").map(|s| s.as_str()).unwrap_or("0");

            let result = if mode == "1" {
                source.chars().skip(position).take(length).collect::<String>()
            } else {
                source.get(position..position + length).unwrap_or("").to_string()
            };
            variables.set(name, Value::String(result));
        }

        "explode" => {
            let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
            let source = get_resolved("source").map(|v| v.as_string()).unwrap_or_default();
            let delimiter = params.get("delimiter").map(|s| s.as_str()).unwrap_or(",");

            let parts: Vec<&str> = source.split(delimiter).collect();
            for (i, part) in parts.iter().enumerate() {
                variables.set(&format!("{}.{}", name, i), Value::String(part.to_string()));
            }
            variables.set(&format!("{}.size", name), Value::Int(parts.len() as i64));
        }

        "date" => {
            let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
            variables.set(&format!("{}.year", name), Value::Int(2024));
            variables.set(&format!("{}.month", name), Value::Int(1));
            variables.set(&format!("{}.day", name), Value::Int(1));
            variables.set(&format!("{}.hour", name), Value::Int(0));
            variables.set(&format!("{}.minute", name), Value::Int(0));
            variables.set(&format!("{}.second", name), Value::Int(0));
        }

        "unixtime" => {
            let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            variables.set(name, Value::Int(ts));
        }

        "os" => {
            let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
            #[cfg(target_os = "windows")]
            let os = "windows";
            #[cfg(target_os = "macos")]
            let os = "macos";
            #[cfg(target_os = "linux")]
            let os = "linux";
            #[cfg(target_os = "ios")]
            let os = "iphone";
            #[cfg(target_os = "android")]
            let os = "android";
            #[cfg(not(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "linux",
                target_os = "ios",
                target_os = "android"
            )))]
            let os = "unknown";
            variables.set(name, Value::String(os.to_string()));
        }

        "fullscreen" | "minimize" => {
            let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
            variables.set(name, Value::Int(0));
        }

        "file_exists" => {
            let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
            let file = params.get("file").map(|s| s.as_str()).unwrap_or("");
            let exists = std::path::Path::new(file).exists();
            variables.set(name, Value::Bool(exists));
        }

        "base64_encode" => {
            let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
            let source = get_resolved("source").map(|v| v.as_string()).unwrap_or_default();
            variables.set(name, Value::String(format!("[base64: {}]", source)));
        }

        "url_encode" => {
            let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
            let source = get_resolved("source").map(|v| v.as_string()).unwrap_or_default();
            let encoded: String = source.chars().map(|c| {
                if c.is_ascii_alphanumeric() || "-_.~".contains(c) {
                    c.to_string()
                } else {
                    format!("%{:02X}", c as u8)
                }
            }).collect();
            variables.set(name, Value::String(encoded));
        }

        "url_decode" => {
            let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
            let source = get_resolved("source").map(|v| v.as_string()).unwrap_or_default();
            let mut decoded = String::new();
            let mut chars = source.chars();
            while let Some(c) = chars.next() {
                if c == '%' {
                    let hex: String = chars.by_ref().take(2).collect();
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        decoded.push(byte as char);
                    }
                } else if c == '+' {
                    decoded.push(' ');
                } else {
                    decoded.push(c);
                }
            }
            variables.set(name, Value::String(decoded));
        }

        "screen_width" | "screen_height" => {
            let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
            let value = if system == "screen_width" { 640 } else { 480 };
            variables.set(name, Value::Int(value));
        }

        "get_layer_info" | "get_sound_info" | "get_backlog_size"
        | "get_backlog_tags" | "get_message_layer_width"
        | "get_message_layer_height" | "get_message_layer_line_width"
        | "get_message_tags" | "get_font" | "get_exe_parameter" => {
            let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
            variables.set(name, Value::Int(0));
        }

        "file_crc" | "file_update_time" | "hmac_sha1_base64"
        | "convert_encoding" | "character_reference_to_utf8"
        | "implode" => {
            let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
            variables.set(name, Value::String(String::new()));
        }

        _ => {}
    }

    Ok(())
}

/// 删除变量树（包括子变量）
fn delete_variable_tree(variables: &mut VariableStore, name: &str) {
    variables.remove(name);

    let prefix = format!("{}.", name);
    let to_remove: Vec<String> = variables.iter_local()
        .chain(variables.iter_global())
        .chain(variables.iter_temp())
        .chain(variables.iter_system())
        .filter(|(k, _)| k.starts_with(&prefix))
        .map(|(k, _)| k.clone())
        .collect();

    for key in to_remove {
        variables.remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_system_var_exist() {
        let mut vars = VariableStore::new();
        vars.set("foo", Value::Int(42));

        let params = {
            let mut m = HashMap::new();
            m.insert("name".to_string(), "result".to_string());
            m.insert("target".to_string(), "foo".to_string());
            m
        };
        let resolved = HashMap::new();

        execute_var_system("var_exist", &params, &resolved, &mut vars).unwrap();
        assert_eq!(vars.get("result"), Some(&Value::Bool(true)));

        let params2 = {
            let mut m = HashMap::new();
            m.insert("name".to_string(), "result".to_string());
            m.insert("target".to_string(), "bar".to_string());
            m
        };
        execute_var_system("var_exist", &params2, &resolved, &mut vars).unwrap();
        assert_eq!(vars.get("result"), Some(&Value::Bool(false)));
    }

    #[test]
    fn test_var_system_explode() {
        let mut vars = VariableStore::new();

        let params = {
            let mut m = HashMap::new();
            m.insert("name".to_string(), "arr".to_string());
            m
        };
        let mut resolved = HashMap::new();
        resolved.insert("source".to_string(), Value::String("a,b,c".to_string()));

        execute_var_system("explode", &params, &resolved, &mut vars).unwrap();

        assert_eq!(vars.get("arr.0"), Some(&Value::String("a".to_string())));
        assert_eq!(vars.get("arr.1"), Some(&Value::String("b".to_string())));
        assert_eq!(vars.get("arr.2"), Some(&Value::String("c".to_string())));
        assert_eq!(vars.get("arr.size"), Some(&Value::Int(3)));
    }

    #[test]
    fn test_var_system_length() {
        let mut vars = VariableStore::new();

        let params = {
            let mut m = HashMap::new();
            m.insert("name".to_string(), "len".to_string());
            m.insert("mode".to_string(), "1".to_string());
            m
        };
        let mut resolved = HashMap::new();
        resolved.insert("source".to_string(), Value::String("hello".to_string()));

        execute_var_system("length", &params, &resolved, &mut vars).unwrap();
        assert_eq!(vars.get("len"), Some(&Value::Int(5)));
    }

    #[test]
    fn test_var_system_find() {
        let mut vars = VariableStore::new();

        let params = {
            let mut m = HashMap::new();
            m.insert("name".to_string(), "pos".to_string());
            m
        };
        let mut resolved = HashMap::new();
        resolved.insert("source".to_string(), Value::String("hello world".to_string()));
        resolved.insert("string".to_string(), Value::String("world".to_string()));

        execute_var_system("find", &params, &resolved, &mut vars).unwrap();
        assert_eq!(vars.get("pos"), Some(&Value::Int(6)));
    }

    #[test]
    fn test_var_system_substr() {
        let mut vars = VariableStore::new();

        let params = {
            let mut m = HashMap::new();
            m.insert("name".to_string(), "sub".to_string());
            m.insert("mode".to_string(), "1".to_string());
            m
        };
        let mut resolved = HashMap::new();
        resolved.insert("source".to_string(), Value::String("hello world".to_string()));
        resolved.insert("position".to_string(), Value::Int(6));
        resolved.insert("length".to_string(), Value::Int(5));

        execute_var_system("substr", &params, &resolved, &mut vars).unwrap();
        assert_eq!(vars.get("sub"), Some(&Value::String("world".to_string())));
    }

    #[test]
    fn test_var_system_delete() {
        let mut vars = VariableStore::new();
        vars.set("foo.bar", Value::Int(1));
        vars.set("foo.baz", Value::Int(2));
        vars.set("foo", Value::Int(3));

        let params = {
            let mut m = HashMap::new();
            m.insert("name".to_string(), "foo".to_string());
            m
        };
        let resolved = HashMap::new();

        execute_var_system("delete", &params, &resolved, &mut vars).unwrap();

        assert_eq!(vars.get("foo"), None);
        assert_eq!(vars.get("foo.bar"), None);
        assert_eq!(vars.get("foo.baz"), None);
    }
}
