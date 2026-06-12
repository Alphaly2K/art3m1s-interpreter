//! 条件标签处理器
//!
//! 实现 if/elseif/else/endif 条件标签和 loop/endloop 循环标签。

use super::{ExecutionContext, TagHandler, TagResult};
use crate::error::Result;

/// 求值 estimate 参数（已经是 $ 开头的表达式）
fn evaluate_estimate(ctx: &ExecutionContext<'_>) -> Result<bool> {
    let condition = ctx.instruction.get("estimate").unwrap_or("1");
    // estimate 值已经是表达式（可能以 $ 开头，也可能是字面量）
    let value = ctx.evaluator().resolve_param(condition)?;
    Ok(value.as_bool())
}

/// [if] 条件标签
pub struct IfHandler;

impl TagHandler for IfHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        if evaluate_estimate(ctx)? {
            Ok(TagResult::Continue)
        } else {
            Ok(TagResult::Jump(ctx.find_else_elseif_or_endif()?))
        }
    }
}

/// [elseif] 条件分支标签
pub struct ElseifHandler;

impl TagHandler for ElseifHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        if evaluate_estimate(ctx)? {
            Ok(TagResult::Continue)
        } else {
            Ok(TagResult::Jump(ctx.find_elseif_else_or_endif()?))
        }
    }
}

/// [else] 标签
pub struct ElseHandler;

impl TagHandler for ElseHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Jump(ctx.find_endif()?))
    }
}

/// [/if] 结束标签
pub struct EndifHandler;

impl TagHandler for EndifHandler {
    fn execute(&self, _ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Continue)
    }
}

/// [loop] 循环标签
pub struct LoopHandler;

impl TagHandler for LoopHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        if evaluate_estimate(ctx)? {
            Ok(TagResult::Continue)
        } else {
            // 条件为假，跳过循环体到 /loop 之后
            Ok(TagResult::Jump(ctx.find_endloop()? + 1))
        }
    }
}

/// [/loop] 结束标签 - 跳回对应的 loop 开始位置
pub struct EndloopHandler;

impl TagHandler for EndloopHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        Ok(TagResult::Jump(ctx.find_loop_start()?))
    }
}
