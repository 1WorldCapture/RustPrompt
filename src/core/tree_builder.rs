use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::AppError;
use anyhow::anyhow;
use super::ignore_rules::IgnoreConfig;

/// 基于 ignore_rules::IgnoreConfig 和 .gitignore 等规则生成项目树字符串
pub fn generate_project_tree_string(root: &Path, ignore_config: &IgnoreConfig) -> Result<String, AppError> {
    let mut lines = Vec::new();

    let root_name = root.file_name()
        .map(|os| os.to_string_lossy().to_string())
        .unwrap_or_else(|| root.display().to_string());
    lines.push(root_name);

    // 使用 ignore_config 创建 walker，它将处理忽略规则
    let walker_for_build = ignore_config.build_walker(root).build();

    // 第一次：收集每个目录的子节点，并计算每个深度的最后条目
    let mut dir_children: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    let mut last_entry_at_depth: HashMap<usize, PathBuf> = HashMap::new();

    // 需要再次构建 walker 来迭代，因为 walker 不能被克隆或重置
    for entry_result in ignore_config.build_walker(root).build() {
        match entry_result {
            Ok(entry) => {
                let path = entry.path().to_path_buf();
                let depth = entry.depth();
                if depth > 0 {
                    if let Some(parent) = path.parent() {
                        dir_children.entry(parent.to_path_buf()).or_default().push(path);
                    }
                }
            }
            Err(err) => return Err(AppError::General(anyhow!("Walk error during collection: {}", err))),
        }
    }

    // 对每个目录的子条目排序，并确定最后一个条目
    for (parent_path, children) in dir_children.iter_mut() {
        children.sort();
        if let Some(last_child) = children.last() {
            // 深度应该是父目录的深度 + 1。父目录的深度可以通过其路径中的组件数计算
            // 根目录 depth 0, 其子节点 depth 1.
            // 父目录的深度是 path.ancestors().count() - 1 (减去自身)
            // 这里使用 parent_path.ancestors().count() 来近似父目录的深度(相对于根目录的层级)
            // 注意：根目录的直接子项深度为 1
             let parent_depth = parent_path.strip_prefix(root).map_or(0, |p| p.components().count());
             last_entry_at_depth.insert(parent_depth + 1, last_child.clone());
        }
    }


    // 第二次：使用 walker_for_build 构造输出字符串
    for result in walker_for_build {
        match result {
            Ok(entry) => {
                let path = entry.path();
                let depth = entry.depth();

                if depth == 0 { // 跳过根目录本身
                    continue;
                }

                let mut prefix = String::new();
                for i in 1..depth {
                    // 检查当前深度 i 是否在其祖先链上的某个最后条目的路径上
                    // 如果 i 深度对应的父路径是其所在层级的最后一个条目，则用空格，否则用 '|'
                    // 我们需要找到深度为 i 的祖先路径
                    let ancestor_at_i = path.ancestors().nth(depth - i).unwrap_or(root);
                    let parent_of_ancestor_at_i = ancestor_at_i.parent().unwrap_or(root);
                    let parent_depth = parent_of_ancestor_at_i.strip_prefix(root).map_or(0, |p| p.components().count());

                    // 检查深度为 i 的祖先是否是其父级（深度 i-1）的最后一个子节点
                    let is_ancestor_last = last_entry_at_depth.get(&(parent_depth + 1))
                                        .map_or(false, |last| last == ancestor_at_i);

                    if is_ancestor_last {
                         prefix.push_str("    "); // 在最后一个分支下
                    } else {
                         prefix.push_str("│   "); // 不在最后一个分支下
                    }
                }

                // 判断当前条目自身是否是其父目录的最后一个子节点
                let is_last = last_entry_at_depth.get(&depth)
                                 .map_or(false, |last| last == path);

                let branch = if is_last { "└── " } else { "├── " };

                let name = path.file_name()
                    .map(|os| os.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());

                lines.push(format!("{}{}{}", prefix, branch, name));
            }
            Err(err) => return Err(AppError::General(anyhow!("Walk error during build: {}", err))),
        }
    }

    Ok(lines.join("\n"))
} 