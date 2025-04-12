// src/core/ignore_rules.rs

use std::path::Path;
use ignore::WalkBuilder;

/// 我们的忽略配置：
/// - 隐藏文件/目录
/// - .gitignore 文件
/// - node_modules (可选)
/// 后续还可以在这里加更多自定义规则
#[derive(Debug, Clone)]
pub struct IgnoreConfig {
    pub ignore_hidden: bool,         // 是否忽略隐藏文件
    pub use_gitignore: bool,         // 是否读取 .gitignore
    pub ignore_node_modules: bool,   // 是否忽略 node_modules
    // 还可加入更多选项
}

impl Default for IgnoreConfig {
    fn default() -> Self {
        Self {
            ignore_hidden: true,
            use_gitignore: true,
            ignore_node_modules: true,
        }
    }
}

impl IgnoreConfig {
    /// 根据我们的 ignore config 构建一个 WalkBuilder
    /// 
    /// `root` : 要扫描的起始目录
    pub fn build_walker(&self, root: &Path) -> WalkBuilder {
        let mut builder = WalkBuilder::new(root);

        // 是否忽略隐藏文件
        builder.hidden(self.ignore_hidden);

        // 是否启用 .gitignore
        if self.use_gitignore {
            builder.git_ignore(true).git_exclude(true).git_global(true);
        } else {
            builder.git_ignore(false).git_exclude(false).git_global(false);
        }
        
        // 简单方式忽略 node_modules: 添加一个忽略模式
        if self.ignore_node_modules {
            // 相对路径模式
            builder.add_ignore("node_modules");
        }

        // 使用标准过滤规则 (如 .git, *.bak 等)
        builder.standard_filters(true);

        builder
    }

    /// 是否忽略单个 Path （可选）
    /// 主要用于自定义DFS/BFS 或单文件判断
    pub fn should_ignore_path(&self, path: &Path) -> bool {
        // 1) 检查是否为隐藏(如果 ignore_hidden=true)
        if self.ignore_hidden {
            if let Some(name) = path.file_name() {
                if name.to_string_lossy().starts_with(".") {
                    return true;
                }
            }
        }

        // 2) 是否是 node_modules
        if self.ignore_node_modules {
            if path.components().any(|c| c.as_os_str() == "node_modules") {
                return true;
            }
        }

        // 注意：这里没有处理 .gitignore，因为这需要解析文件。
        // WalkBuilder 已经处理了 .gitignore。
        // 如果你需要完全独立的判断，需要引入 gitignore 解析库。

        false // 默认不忽略
    }
} 