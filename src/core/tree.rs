// src/core/tree.rs

use std::path::{Path, PathBuf};
use crate::error::AppError;
use anyhow::anyhow;
use super::ignore_rules::IgnoreConfig;
use std::collections::HashMap;

pub fn generate_project_tree_string(root: &Path, ignore_config: &IgnoreConfig) -> Result<String, AppError> {
    let mut lines = Vec::new();
    let root_name = root
        .file_name()
        .map(|os| os.to_string_lossy().to_string())
        .unwrap_or_else(|| root.display().to_string());

    lines.push(root_name);

    // 使用 WalkBuilder 进行遍历
    let walker = ignore_config.build_walker(root).build();
    // 存储每个深度的最后一个条目的路径，用于判断是否使用 '└──'
    let mut last_entry_at_depth: HashMap<usize, PathBuf> = HashMap::new(); 
    // 预先计算每个目录的子条目，以确定最后一个条目
    let mut dir_children: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

    // 第一遍遍历: 收集目录结构信息
    for result in ignore_config.build_walker(root).build() {
        match result {
            Ok(entry) => {
                let path = entry.path().to_path_buf();
                let depth = entry.depth(); // depth 0 is the root itself
                if depth > 0 { // 只关心根目录的子节点
                    if let Some(parent) = path.parent() {
                        dir_children.entry(parent.to_path_buf()).or_default().push(path);
                    }
                }
            }
            Err(err) => return Err(AppError::General(anyhow!("Walk error: {}", err))),
        }
    }
    // 对每个目录的子条目排序
    for children in dir_children.values_mut() {
        children.sort();
        if let Some(last) = children.last() {
            if let Some(parent) = last.parent() {
                 last_entry_at_depth.insert(parent.ancestors().count() -1 , last.clone()); // depth is parent depth + 1
            }
           
        }
    }


    // 第二遍遍历: 构建输出字符串
    for result in walker {
        match result {
            Ok(entry) => {
                let path = entry.path();
                let depth = entry.depth();

                if depth == 0 { // 跳过根目录本身，因为它已经添加到 lines[0]
                    continue;
                }

                let mut prefix = String::new();
                for i in 1..depth {
                    // 判断当前深度是否是其父级最后一个条目的路径的一部分
                    // 如果是，并且父级更深处还有最后一个元素，则用空格，否则用 '│'
                    let parent_depth = i;
                    // Check if any ancestor path at this depth is a last entry
                    let is_on_last_path = path.ancestors().take(depth - parent_depth +1).any(|ancestor|
                         last_entry_at_depth.get(&parent_depth) == Some(&ancestor.to_path_buf())
                    );

                    if is_on_last_path {
                         prefix.push_str("    "); // 在最后一个分支下
                    } else {
                         prefix.push_str("│   "); // 不在最后一个分支
                    }

                }
                
                // 获取当前条目的父目录的子条目列表
                let parent_path = path.parent().unwrap_or(root).to_path_buf();
                let siblings = dir_children.get(&parent_path).cloned().unwrap_or_default();
                let is_last = siblings.last().map_or(false, |last| last == path);

                let branch = if is_last { "└── " } else { "├── " };

                let name = path.file_name()
                    .map(|os| os.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());

                lines.push(format!("{}{}{}", prefix, branch, name));
            }
            Err(err) => return Err(AppError::General(anyhow!("Walk error: {}", err))),
        }
    }

    Ok(lines.join("\n"))
}

// 移除旧的 build_tree_recursive 函数
/*
fn build_tree_recursive(
    path: &Path,
    prefix: String,
    lines: &mut Vec<String>,
    ignore_config: &IgnoreConfig
) -> Result<(), AppError> {
    // ... old implementation ...
}
*/ 