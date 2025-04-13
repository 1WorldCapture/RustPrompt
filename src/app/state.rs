use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// 虚拟路径常量，用作项目目录树的唯一 key
pub const PROJECT_TREE_VIRTUAL_PATH: &str = "__PROJECT_TREE__";

#[derive(Debug, Clone, PartialEq)]
pub enum ReplMode {
    Manual,
    Prompt,
}

/// 用于区分 REPL 编辑器的状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReplEditorMode {
    SingleLine,
    MultiLine,
}

/// 全局共享状态
pub struct AppState {
    /// 已选中的真实文件路径
    pub selected_paths: HashSet<PathBuf>,

    /// 已选文件数
    pub file_count: usize,

    /// 当前合并后 XML 的 Token 数
    pub token_count: usize,

    /// 每个「文件」(包括虚拟文件) -> 其 `<document index="x"> ... </document>` 片段
    pub partial_docs: HashMap<PathBuf, String>,

    /// 最终合并得到的完整XML
    pub cached_xml: String,

    /// 当前模式: manual or prompt
    pub mode: ReplMode,

    /// prompt模式下收集到的提示词 (可多行或单行)
    pub prompt_text: String,

    /// 编辑器模式：单行或多行
    pub editor_mode: ReplEditorMode,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            selected_paths: HashSet::new(),
            file_count: 0,
            token_count: 0,
            partial_docs: HashMap::new(),
            cached_xml: String::new(),
            mode: ReplMode::Manual,
            prompt_text: String::new(),
            editor_mode: ReplEditorMode::SingleLine,
        }
    }
} 