use std::collections::HashSet;
use std::path::PathBuf;

/// 全局共享状态
pub struct AppState {
    /// 已选中的文件路径（未来可扩展为去重、归一化等逻辑）
    pub selected_paths: HashSet<PathBuf>,

    /// 已选文件数
    pub file_count: usize,

    /// Token 数量（占位，后续 Sprint 里会真正计算）
    pub token_count: usize,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            selected_paths: HashSet::new(),
            file_count: 0,
            token_count: 0,
        }
    }
} 