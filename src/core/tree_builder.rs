use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::AppError;
use anyhow::anyhow;
use super::ignore_rules::IgnoreConfig;

/// 基于 ignore_rules::IgnoreConfig 和 .gitignore 等规则生成项目树字符串
pub fn generate_project_tree_string(root: &Path, ignore_config: &IgnoreConfig) -> Result<String, AppError> {
    // 1) 使用 ignore_config.build_walker 收集所有符合规则的条目
    let mut entries = Vec::new();
    let walker = ignore_config.build_walker(root).build();

    for result in walker {
        let entry = result.map_err(|e|
            AppError::General(anyhow!("项目树扫描失败: {:?}", e))
        )?;
        // 只收集深度大于0的，根目录后面单独处理
        if entry.depth() > 0 {
             entries.push(entry);
        }
    }

    // 2) 构建一个 parent -> [children] 的映射，用以表示层级
    //    这里用 BTreeMap 方便后续稳定排序 (按路径排序)
    let mut children_map: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    // 同时记录哪些路径本身也在列表中 (主要为了判断子节点是否是目录且被扫描到)
    let mut is_entry_in_list = BTreeMap::new();

    // 插入根目录本身，以启动 DFS
    is_entry_in_list.insert(root.to_path_buf(), 0); // depth 0

    for e in &entries {
        let path = e.path().to_path_buf();
        let depth = e.depth();
        is_entry_in_list.insert(path.clone(), depth);

        if depth > 0 { // 确保 parent 不是根目录自身
            if let Some(parent) = path.parent() {
                // 只有当 parent 也在扫描结果中 (或 parent 是 root) 时才添加
                // 这确保我们不会为被忽略的目录添加子节点
                if parent == root || is_entry_in_list.contains_key(parent) {
                     children_map.entry(parent.to_path_buf()).or_default().push(path);
                }
            }
        }
    }

    // 3) 对每个 parent 的子列表进行排序
    for (_k, v) in children_map.iter_mut() {
        v.sort();
    }

    // 4) 递归构造树状输出
    let mut lines = Vec::new();

    // 确认 root 的名字
    let root_name = root.file_name()
        .map(|os| os.to_string_lossy().to_string())
        .unwrap_or_else(|| root.display().to_string());
    lines.push(root_name);

    // 定义一个递归函数
    fn dfs_build(
        current: &PathBuf,
        prefix: String,
        lines: &mut Vec<String>,
        children_map: &BTreeMap<PathBuf, Vec<PathBuf>>,
        // is_entry_in_list: &BTreeMap<PathBuf, usize>, // 不需要此参数，map里有就说明被扫描
    ) {
        // 拿到此路径的子列表
        let children = match children_map.get(current) {
            Some(c) => c,
            None => return, // 没有子节点
        };

        let total = children.len();
        for (i, child) in children.iter().enumerate() {
            let is_last = i == total - 1;
            let branch = if is_last { "└── " } else { "├── " };

            let name = child.file_name()
                .map(|os| os.to_string_lossy().to_string())
                .unwrap_or_else(|| child.display().to_string());

            let line = format!("{}{}{}", prefix, branch, name);
            lines.push(line);

            // 如果 child 也是一个目录 (存在于 children_map 的 key 中)，则继续递归
            if children_map.contains_key(child) {
                // 继续向下构造
                let ext_prefix = if is_last {
                    format!("{}    ", prefix) // 最后一个用 4空格
                } else {
                    format!("{}│   ", prefix) // 不是最后一个用 "│   "
                };
                dfs_build(child, ext_prefix, lines, children_map);
            }
        }
    }

    // 5) 调用 DFS，从 root(深度0)开始
    let root_key = root.to_path_buf();
    dfs_build(&root_key, "".to_string(), &mut lines, &children_map);

    // 6) 拼装结果
    Ok(lines.join("\n"))
} 