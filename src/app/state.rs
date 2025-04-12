use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// 虚拟路径常量，用作项目目录树的唯一 key
pub const PROJECT_TREE_VIRTUAL_PATH: &str = "__PROJECT_TREE__";

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
}

impl AppState {
    pub fn new() -> Self {
        Self {
            selected_paths: HashSet::new(),
            file_count: 0,
            token_count: 0,
            partial_docs: HashMap::new(),
            cached_xml: String::new(),
        }
    }
} 