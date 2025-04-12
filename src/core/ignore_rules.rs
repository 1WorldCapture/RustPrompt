// src/core/ignore_rules.rs

use std::path::Path;
use ignore::WalkBuilder;

/// 忽略配置：管理隐藏文件/.gitignore/node_modules等
#[derive(Debug, Clone)]
pub struct IgnoreConfig {
    pub ignore_hidden: bool,
    pub use_gitignore: bool,
    pub ignore_node_modules: bool,
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

        builder.hidden(self.ignore_hidden);

        if self.use_gitignore {
            builder.git_ignore(true).git_exclude(true).git_global(true);
        } else {
            builder.git_ignore(false).git_exclude(false).git_global(false);
        }
        if self.ignore_node_modules {
            // 添加忽略模式，确保在所有子目录中都生效
            builder.add_ignore("**/node_modules");
            builder.add_ignore("node_modules"); // 也覆盖根目录下的
        }
        builder.standard_filters(true);

        builder
    }

    /// 检查单个路径是否应该被忽略 (基于配置，但不解析 .gitignore)
    pub fn should_ignore_path(&self, path: &Path) -> bool {
        if self.ignore_hidden {
            if let Some(name) = path.file_name() {
                if name.to_string_lossy().starts_with('.') {
                    // 对于 Unix 隐藏文件
                    return true;
                }
            }
            // 可选: 添加 Windows 隐藏文件检查 (需要额外 crate 或 cfg)
        }
        if self.ignore_node_modules {
            if path.components().any(|c| c.as_os_str() == "node_modules") {
                return true;
            }
        }
        // 注意: 此方法不处理 .gitignore。完整的忽略判断依赖于 WalkBuilder
        false
    }
} 