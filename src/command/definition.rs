use std::path::PathBuf;

/// 我们支持的命令列表
#[derive(Debug, Clone)] // 确保 Clone trait 已添加
pub enum Command {
    Add(PathBuf),
    Remove(PathBuf),
    ShowContext,
    Copy,
    Reset,
    Help,
    Quit,
    Unknown(String),

    // 新增: 模式切换 /mode
    Mode(Option<String>), // None表示查看当前模式；Some("manual"/"prompt")表示切换

    // 新增: 查看/编辑 prompt_text
    Prompt, // /prompt => 打印当前 prompt_text

    // 如果在 prompt 模式下输入普通行，会转换成此命令
    AppendPromptText(String),
} 