//! 事件系统
//!
//! 定义解释器在执行过程中产生的各种事件，
//! 以及用户通过回调处理这些事件的机制。

use std::collections::HashMap;

/// 解释器事件
#[derive(Debug, Clone)]
pub enum Event {
    /// UI 转场效果
    UiTransition(TransitionEvent),

    /// 图层操作
    Layer(LayerEvent),

    /// 加载状态变化
    LoadingState {
        /// 是否激活
        active: bool,
    },

    /// 存档状态变化
    SavingState {
        /// 是否激活
        active: bool,
    },

    /// 音效播放
    PlaySound {
        /// 音效名称
        name: String,
        /// 是否等待播放完成
        wait: bool,
    },

    /// 停止所有音效
    StopAllSounds {
        /// 淡出时间（毫秒）
        duration: u64,
    },

    /// 对话框显示
    ShowDialog {
        /// 标题
        title: String,
        /// 消息内容
        message: String,
        /// 接收确定/取消结果（1/0）的变量名
        varname: Option<String>,
        /// 接收文本输入的变量名
        textfield: Option<String>,
        /// 文本输入最大字符数
        textfield_size: Option<usize>,
    },

    /// 是/否选择
    YesNo {
        /// 配置文件
        file: String,
        /// 音效
        se: Option<String>,
    },

    /// 脚本调用（call）
    ScriptCall {
        /// 脚本文件
        file: String,
        /// 标签名
        label: String,
    },

    /// 退出请求
    Exit,

    /// 返回标题
    GoTitle,

    /// 通用等待
    Wait {
        /// 等待原因
        reason: WaitReason,
    },

    /// 剧情文本
    Text {
        /// 文本内容
        content: String,
    },

    /// 加载遮罩
    LoadMask {
        /// 操作类型
        action: LoadMaskAction,
        /// 时间（毫秒）
        time: u64,
    },

    /// 全部删除
    AllDelete {
        /// 时间（毫秒）
        time: u64,
    },

    /// 系统 UI 显示/隐藏
    SystemUi {
        /// 操作类型
        action: SystemUiAction,
    },

    /// 存档操作
    SaveOperation {
        /// 操作类型
        action: SaveAction,
    },

    /// 重复执行标记
    Repeatedly,

    /// 自动跳过禁用
    AutoSkipDisable,

    // ── 剧情文本事件 ──────────────────────────────────────
    /// 剧情文本
    ScenarioText { content: String, inline: bool },
    /// 换行 [rt]
    LineBreak,
    /// 分页 [rp]
    PageBreak { backlog: Option<i32> },
    /// 字体设置 [font]
    FontSettings(HashMap<String, String>),
    /// 回退字体 [font_close]
    FontClose,
    /// 默认字体设置 [fontdefault]
    FontDefault(HashMap<String, String>),
    /// 初始化字体 [fontinit]
    FontInit,
    /// 开始注音 [ruby]
    RubyStart { text: String },
    /// 结束注音 [/ruby]
    RubyEnd,
    /// 开始链接 [link]
    LinkStart {
        file: Option<String>,
        label: Option<String>,
        link_type: i32,
        color: Option<String>,
    },
    /// 结束链接 [/link]
    LinkEnd,
    /// 禁用链接
    LinkDisable,
    /// 启用链接
    LinkEnable,
    /// 点击等待图标 [glyph]
    GlyphConfig(HashMap<String, String>),
    /// 切换消息层 [chgmsg]
    MessageLayerSwitch { id: Option<String>, layered: i32 },
    /// 回退消息层 [chgmsg_close]
    MessageLayerPop,
    /// 文本动画 [scetween]
    TextAnimation(HashMap<String, String>),
    /// 场景进入 [scein]
    SceneIn,
    /// 场景退出 [sceout]
    SceneOut,
    /// 自动模式配置 [automode]
    AutoModeConfig { allow: bool, layer: Option<String> },
    /// 跳过配置 [skip]
    SkipConfig { allow: bool, skip_unread: bool },
    /// 历史配置 [backlog]
    BacklogConfig { allow: bool },
    /// 隐藏配置 [hide]
    HideConfig { allow: bool },
    /// 已读判定 [alreadyread]
    AlreadyReadConfig { mode: i32 },
    /// 历史写入 [writebacklog]
    WriteBacklogConfig { enable: bool },
    /// 缩进设置 [indent]
    IndentConfig { value: String },
    /// 禁则处理 [prohibit]
    ProhibitConfig { value: String },
    /// 单词部分字符 [wordparts]
    WordpartsConfig { value: String },

    // ── 音频事件 ──────────────────────────────────────
    /// 播放 BGM [splay]
    BgmPlay {
        file: String,
        loop_play: bool,
        gain: Option<i32>,
        pan: Option<i32>,
        fade_time: Option<u64>,
    },
    /// 停止 BGM [sstop]
    BgmStop { fade_time: Option<u64> },
    /// BGM 音量渐变 [sfade]
    BgmFade { gain: i32, time: u64 },
    /// BGM 声像 [span]
    BgmPan { pan: i32 },
    /// BGM 交叉淡入 [sxfade]
    BgmCrossFade {
        file: String,
        loop_play: bool,
        gain: Option<i32>,
        pan: Option<i32>,
        time: u64,
    },
    /// 播放 SE [seplay]
    SePlay {
        id: String,
        file: String,
        loop_play: bool,
        gain: Option<i32>,
        pan: Option<i32>,
        fade_time: Option<u64>,
        skippable: bool,
    },
    /// 停止 SE [sestop]
    SeStop { id: String, fade_time: Option<u64> },
    /// SE 音量渐变 [sefade]
    SeFade { id: String, gain: i32, time: u64 },
    /// SE 声像 [sepan]
    SePan { id: String, pan: i32 },
    /// 语音播放 [voice]
    VoicePlay {
        file: String,
        gain: Option<i32>,
        pan: Option<i32>,
        fade_time: Option<u64>,
    },
    /// 音效完成事件处理器 [setonsoundfinish]
    SoundFinishHandler {
        id: String,
        file: Option<String>,
        label: Option<String>,
        call: bool,
        handler: Option<String>,
    },
    /// 解除音效完成事件处理器 [delonsoundfinish]
    SoundFinishHandlerDel { id: String },

    // ── 系统操作事件 ──────────────────────────────────────
    /// 执行用户操作 [exec]
    Exec { command: String, mode: Option<i32> },
    /// 存档 [save]
    SaveGame { file: String },
    /// 读档 [load]
    LoadGame {
        file: String,
        trans_type: Option<i32>,
    },
    /// 调试设置 [debug]
    DebugConfig {
        mode: Option<i32>,
        level: Option<i32>,
    },
    /// 调试输出 [debugprint]
    DebugPrint { level: i32, data: String },
    /// 调试重载 [debugreload]
    DebugReload,
    /// 窗口标题 [caption]
    Caption { data: String },
    /// 鼠标设置 [mouse]
    MouseConfig {
        left: Option<i32>,
        top: Option<i32>,
        hide: Option<i32>,
        autohide: Option<u64>,
    },
    /// 按键配置 [keyconfig]
    KeyConfig(HashMap<String, String>),
    /// 文件操作 [file]
    FileOperation {
        command: String,
        src: Option<String>,
        dst: Option<String>,
        target: Option<String>,
    },
    /// HTTP GET [httpget]
    HttpGet { url: String },
    /// HTTP POST [httppost]
    HttpPost {
        url: String,
        params: HashMap<String, String>,
    },
    /// 打开浏览器 [openbrowser]
    OpenBrowser { url: String },
    /// 自动存档配置 [autosave]
    AutoSaveConfig { allow: bool },
    /// 紧急回避配置 [avoid]
    AvoidConfig { allow: bool },
    /// 振动 [vibrate]
    Vibrate { time: u64 },
    /// 状态栏 [statusbar]
    StatusBar { visible: bool },
    /// 应用内购买 [purchase]
    Purchase { item: String },
    /// 调用原生代码 [callnative]
    CallNative {
        function: String,
        params: HashMap<String, String>,
    },

    // ── 事件处理器事件 ──────────────────────────────────────
    /// 设置事件处理器 [seton*]
    SetEventHandler {
        event_name: String,
        file: Option<String>,
        label: Option<String>,
        call: bool,
        handler: Option<String>,
        /// 标签里除已知字段外的其它参数（key、adv、ui、btn 等），
        /// 由宿主在事件触发时作为 param 传给 Lua 回调。
        extra_params: std::collections::HashMap<String, String>,
    },
    /// 解除事件处理器 [delon*]
    DelEventHandler {
        event_name: String,
        /// 指定 key 时只解除该键的处理器；None 时解除整个事件类型的所有处理器。
        key: Option<String>,
    },

    // ── 图层缓动 ──────────────────────────────────────
    /// 图层属性缓动 [lytween]
    LayerTween {
        id: String,
        param: String,
        from: Option<String>,
        to: Option<String>,
        ease: Option<String>,
        time: Option<u64>,
        delay: Option<u64>,
        loop_count: Option<i32>,
        yoyo: Option<i32>,
        loop_delay: Option<u64>,
        sync: bool,
        delete: bool,
        handler_file: Option<String>,
        handler_label: Option<String>,
        handler_handler: Option<String>,
    },
    /// 强制完成图层缓动 [lytweendel]
    LayerTweenDelete { id: String },
    /// 缓动序列开始 [tweenset]
    TweenSetStart,
    /// 缓动序列结束 [/tweenset]
    TweenSetEnd,

    // ── 图层事件/操作 ──────────────────────────────────────
    /// 图层事件处理器 [lyevent]
    LayerEventHandler {
        id: String,
        event_type: String,
        mode: String,
        file: Option<String>,
        label: Option<String>,
        call: bool,
        handler: Option<String>,
        penetration: bool,
        /// 标签里除已知字段外的其它参数（name、key、se、function 等），
        /// 触发事件时由宿主原样塞进 handler 标签的参数表。
        extra_params: std::collections::HashMap<String, String>,
    },
    /// 图层重命名 [lyrename]
    LayerRename { id: String, to: String },
    /// 图层图像编辑 [lyedit]
    LayerEdit {
        id: String,
        mode: String,
        color: Option<String>,
        file: Option<String>,
        left: Option<i32>,
        top: Option<i32>,
    },
    /// 图层拖动 [lydrag]
    LayerDrag { id: String },

    // ── 动画/视频 ──────────────────────────────────────
    /// 帧动画 [anime]
    Anime {
        id: String,
        mode: String,
        file: Option<String>,
        mask: Option<String>,
        time: Option<u64>,
        loop_count: Option<i32>,
        props: HashMap<String, String>,
    },
    /// 视频播放 [video]
    VideoPlay {
        id: Option<String>,
        file: String,
        skip: bool,
        loop_play: bool,
    },
    /// 视频播放完成事件处理器 [setonvideofinish]
    VideoFinishHandler {
        file: Option<String>,
        label: Option<String>,
        call: bool,
        handler: Option<String>,
    },
    /// 解除视频播放完成事件处理器 [delonvideofinish]
    VideoFinishHandlerDel,

    // ── 转场/截图 ──────────────────────────────────────
    /// 图层树转换 [trans]
    Trans {
        trans_type: i32,
        time: Option<u64>,
        rule: Option<String>,
        vague: Option<i32>,
        input: i32,
    },
    /// 立即反映图层变更 [flip]
    Flip,
    /// 注册图层 HLSL shader [lyshader]
    ShaderLoad { id: String, file: String },
    /// 截图 [takess]
    TakeScreenshot,
    /// 保存截图 [savess]
    SaveScreenshot {
        file: String,
        width: Option<u32>,
        height: Option<u32>,
    },

    // ── 脚本/宏 ──────────────────────────────────────
    /// 右键单击脚本 [rclick]
    RightClickConfig { allow: bool, file: Option<String> },
    /// 解除宏定义文件 [macrodel]
    MacroDel { file: String },

    /// 自定义事件（未识别的标签）
    Custom {
        /// 标签名
        tag: String,
        /// 参数
        params: HashMap<String, String>,
    },
}

/// UI 转场事件
#[derive(Debug, Clone)]
pub struct TransitionEvent {
    /// 转场时间（毫秒）
    pub time: u64,
    /// 转场类型
    pub fade: Option<String>,
}

/// 图层事件
#[derive(Debug, Clone)]
pub enum LayerEvent {
    /// 创建图层
    Create { id: String, file: String },
    /// 创建图层（变体）
    Create2 {
        id: String,
        file: String,
        alpha: Option<u8>,
    },
    /// 删除图层
    Delete { id: String },
    /// 设置图层属性
    SetProperty {
        id: String,
        property: String,
        value: String,
    },
    /// 批量设置图层属性（用于图层集）
    SetProperties {
        id: String,
        properties: HashMap<String, String>,
    },
}

/// 图层属性
#[derive(Debug, Clone, Default)]
pub struct LayerProperties {
    /// 左边位置
    pub left: Option<i32>,
    /// 顶部位置
    pub top: Option<i32>,
    /// 宽度
    pub width: Option<i32>,
    /// 高度
    pub height: Option<i32>,
    /// 透明度 (0-255)
    pub alpha: Option<u8>,
    /// 是否可见
    pub visible: Option<bool>,
    /// 缩放 X
    pub scale_x: Option<f32>,
    /// 缩放 Y
    pub scale_y: Option<f32>,
    /// 旋转角度
    pub rotation: Option<f32>,
    /// 其他自定义属性
    pub custom: HashMap<String, String>,
}

/// 等待原因
#[derive(Debug, Clone)]
pub enum WaitReason {
    /// 通用等待 [wt]
    Generic,
    /// 通用等待（变体）[wt0]
    Generic0,
    /// 停止 [stop]
    Stop {
        /// 停止原因
        reason: Option<String>,
    },
    /// 按键等待 [exkey]
    KeyWait {
        /// 按钮列表
        buttons: Vec<String>,
    },
    /// 时间等待 [wait time="xxx" input="x"]
    Timed {
        /// 毫秒数
        milliseconds: u64,
        /// 输入策略：0=不接受输入，1=输入解除等待，2=跳过中不停止
        input: i32,
    },
}

/// 加载遮罩操作
#[derive(Debug, Clone)]
pub enum LoadMaskAction {
    /// 显示
    Show,
    /// 删除
    Delete,
}

/// 系统 UI 操作
#[derive(Debug, Clone)]
pub enum SystemUiAction {
    /// 显示
    Show,
    /// 隐藏
    Hide {
        /// 跳过模式
        skip: Option<String>,
    },
}

/// 存档操作
#[derive(Debug, Clone)]
pub enum SaveAction {
    /// 系统存档
    SystemSave,
    /// 系统读档
    SystemLoad,
}

/// 回调结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackResult {
    /// 继续执行
    Continue,
    /// 暂停执行（等待外部事件）
    Pause,
    /// 中止执行
    Abort,
}

/// 事件回调函数类型
pub type EventCallback = Box<dyn FnMut(Event) -> CallbackResult + Send + Sync>;

/// 脚本加载器函数类型（返回文本）
pub type ScriptLoader = Box<dyn Fn(&str) -> crate::error::Result<String> + Send + Sync>;

/// 脚本文件加载器函数类型（返回原始字节，支持文本和二进制）
pub type ScriptFileLoader = Box<dyn Fn(&str) -> crate::error::Result<Vec<u8>> + Send + Sync>;

/// 默认回调，继续执行所有事件
pub fn default_callback(_event: Event) -> CallbackResult {
    CallbackResult::Continue
}
