//! 音频标签处理器
//!
//! 实现 splay, sstop, sfade, span, sxfade, seplay, sestop, sefade, sepan, voice 等音频相关标签。

use super::{ExecutionContext, TagHandler, TagResult};
use crate::error::Result;
use crate::event::Event;

/// [splay] 播放 BGM
pub struct SplayHandler;

impl TagHandler for SplayHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let file = ctx.resolve_param("file")?.as_string();
        let loop_play = ctx
            .instruction
            .get("loop")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1)
            != 0;
        let gain = ctx
            .instruction
            .get("gain")
            .and_then(|v| v.parse::<i32>().ok());
        let pan = ctx
            .instruction
            .get("pan")
            .and_then(|v| v.parse::<i32>().ok());
        let time = ctx
            .instruction
            .get("time")
            .and_then(|v| v.parse::<u64>().ok());

        Ok(TagResult::Emit(Event::BgmPlay {
            file,
            loop_play,
            gain,
            pan,
            fade_time: time,
        }))
    }
}

/// [sstop] 停止 BGM
pub struct SstopHandler;

impl TagHandler for SstopHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let time = ctx
            .instruction
            .get("time")
            .and_then(|v| v.parse::<u64>().ok());
        Ok(TagResult::Emit(Event::BgmStop { fade_time: time }))
    }
}

/// [sfade] BGM 音量渐变
pub struct SfadeHandler;

impl TagHandler for SfadeHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let gain = ctx.resolve_param("gain")?.as_int().unwrap_or(0) as i32;
        let time = ctx
            .instruction
            .get("time")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        Ok(TagResult::Emit(Event::BgmFade { gain, time }))
    }
}

/// [span] BGM 声像
pub struct SpanHandler;

impl TagHandler for SpanHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let pan = ctx.resolve_param("pan")?.as_int().unwrap_or(0) as i32;
        Ok(TagResult::Emit(Event::BgmPan { pan }))
    }
}

/// [sxfade] 交叉淡入 BGM
pub struct SxfadeHandler;

impl TagHandler for SxfadeHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let file = ctx.resolve_param("file")?.as_string();
        let loop_play = ctx
            .instruction
            .get("loop")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1)
            != 0;
        let gain = ctx
            .instruction
            .get("gain")
            .and_then(|v| v.parse::<i32>().ok());
        let pan = ctx
            .instruction
            .get("pan")
            .and_then(|v| v.parse::<i32>().ok());
        let time = ctx
            .instruction
            .get("time")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        Ok(TagResult::Emit(Event::BgmCrossFade {
            file,
            loop_play,
            gain,
            pan,
            time,
        }))
    }
}

/// [seplay] 播放 SE
pub struct SeplayHandler;

impl TagHandler for SeplayHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.resolve_param_str("id")?;
        let file = ctx.resolve_param("file")?.as_string();
        let loop_play = ctx
            .instruction
            .get("loop")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0)
            != 0;
        let gain = ctx
            .instruction
            .get("gain")
            .and_then(|v| v.parse::<i32>().ok());
        let pan = ctx
            .instruction
            .get("pan")
            .and_then(|v| v.parse::<i32>().ok());
        let time = ctx
            .instruction
            .get("time")
            .and_then(|v| v.parse::<u64>().ok());
        let skippable = ctx
            .instruction
            .get("skippable")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0)
            != 0;

        Ok(TagResult::Emit(Event::SePlay {
            id,
            file,
            loop_play,
            gain,
            pan,
            fade_time: time,
            skippable,
        }))
    }
}

/// [sestop] 停止 SE
pub struct SestopHandler;

impl TagHandler for SestopHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.resolve_param_str("id")?;
        let time = ctx
            .instruction
            .get("time")
            .and_then(|v| v.parse::<u64>().ok());
        Ok(TagResult::Emit(Event::SeStop {
            id,
            fade_time: time,
        }))
    }
}

/// [sefade] SE 音量渐变
pub struct SefadeHandler;

impl TagHandler for SefadeHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.resolve_param_str("id")?;
        let gain = ctx.resolve_param("gain")?.as_int().unwrap_or(0) as i32;
        let time = ctx
            .instruction
            .get("time")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        Ok(TagResult::Emit(Event::SeFade { id, gain, time }))
    }
}

/// [sepan] SE 声像
pub struct SepanHandler;

impl TagHandler for SepanHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.resolve_param_str("id")?;
        let pan = ctx.resolve_param("pan")?.as_int().unwrap_or(0) as i32;
        Ok(TagResult::Emit(Event::SePan { id, pan }))
    }
}

/// [voice] 语音播放
pub struct VoiceHandler;

impl TagHandler for VoiceHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let file = ctx.resolve_param("file")?.as_string();
        let gain = ctx
            .instruction
            .get("gain")
            .and_then(|v| v.parse::<i32>().ok());
        let pan = ctx
            .instruction
            .get("pan")
            .and_then(|v| v.parse::<i32>().ok());
        let time = ctx
            .instruction
            .get("time")
            .and_then(|v| v.parse::<u64>().ok());
        Ok(TagResult::Emit(Event::VoicePlay {
            file,
            gain,
            pan,
            fade_time: time,
        }))
    }
}

/// [setonsoundfinish] 音效完成事件处理
pub struct SetOnSoundFinishHandler;

impl TagHandler for SetOnSoundFinishHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.resolve_param_str("id")?;
        let file = ctx.instruction.get("file").map(String::from);
        let label = ctx.instruction.get("label").map(String::from);
        let call = ctx
            .instruction
            .get("call")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0)
            != 0;
        let handler = ctx.instruction.get("handler").map(String::from);

        Ok(TagResult::Emit(Event::SoundFinishHandler {
            id,
            file,
            label,
            call,
            handler,
        }))
    }
}

/// [delonsoundfinish] 解除音效完成事件处理
pub struct DelOnSoundFinishHandler;

impl TagHandler for DelOnSoundFinishHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.resolve_param_str("id")?;
        Ok(TagResult::Emit(Event::SoundFinishHandlerDel { id }))
    }
}

/// [sefadein] SE 淡入（已弃用）
pub struct SefadeinHandler;

impl TagHandler for SefadeinHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.resolve_param_str("id")?;
        let time = ctx
            .instruction
            .get("time")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        Ok(TagResult::Emit(Event::SeFade {
            id,
            gain: 1000,
            time,
        }))
    }
}

/// [sefadeout] SE 淡出（已弃用）
pub struct SefadeoutHandler;

impl TagHandler for SefadeoutHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let id = ctx.resolve_param_str("id")?;
        let time = ctx
            .instruction
            .get("time")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        Ok(TagResult::Emit(Event::SeFade { id, gain: 0, time }))
    }
}

/// [sfadein] BGM 淡入（已弃用）
pub struct SfadeinHandler;

impl TagHandler for SfadeinHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let time = ctx
            .instruction
            .get("time")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        Ok(TagResult::Emit(Event::BgmFade { gain: 1000, time }))
    }
}

/// [sfadeout] BGM 淡出（已弃用）
pub struct SfadeoutHandler;

impl TagHandler for SfadeoutHandler {
    fn execute(&self, ctx: &mut ExecutionContext<'_>) -> Result<TagResult> {
        let time = ctx
            .instruction
            .get("time")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        Ok(TagResult::Emit(Event::BgmFade { gain: 0, time }))
    }
}
