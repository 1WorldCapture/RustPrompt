use std::path::PathBuf;

/// 我们支持的命令列表
#[derive(Debug, Clone)] // 确保 Clone trait 已添加
pub enum Command {
    Add(PathBuf),
    Remove(PathBuf),
    ShowContext,
    Copy,
    Help,
    Quit,
    Unknown(String),
} 