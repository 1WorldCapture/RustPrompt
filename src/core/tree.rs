// src/core/tree.rs

use std::path::{Path, PathBuf};
use std::fs;

use crate::error::AppError;
use anyhow::anyhow;

pub fn generate_project_tree_string(root: &Path) -> Result<String, AppError> {
    // 为了简单，我们只递归一层或几层，或者做一个深度递归。
    // 这里示例：递归多层，输出"├──"和"└──"结构。
    // 也可以使用现有 crate，例如 "tree" 或者自写 BFS/DFS。
    let mut lines = Vec::new();
    let root_name = root
        .file_name()
        .map(|os| os.to_string_lossy().to_string())
        .unwrap_or_else(|| root.display().to_string());

    lines.push(root_name); // 第一个行，显示根目录本身的名字

    // 开始构建树
    build_tree_recursive(root, "".to_string(), &mut lines)?;

    Ok(lines.join("\n"))
}

/// 递归辅助函数
fn build_tree_recursive(path: &Path, prefix: String, lines: &mut Vec<String>) -> Result<(), AppError> {
    if !path.is_dir() {
        return Ok(()); // 如果不是目录，就不再展开
    }

    let entries = fs::read_dir(path)
        .map_err(|e| AppError::General(anyhow!("无法读取目录 {:?}: {:?}", path, e)))?;
    
    // 收集并排序，让文件夹和文件按名字排列
    let mut children: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|x| x.path())).collect();
    children.sort();

    let count = children.len();
    for (i, child) in children.into_iter().enumerate() {
        let is_last = i == count - 1;
        let child_name = child.file_name()
                              .map(|os| os.to_string_lossy().to_string())
                              .unwrap_or_else(|| child.display().to_string());

        let branch = if is_last { "└── " } else { "├── " };
        lines.push(format!("{}{}{}", prefix, branch, child_name));

        // 如果是文件夹，需要继续递归
        if child.is_dir() {
            // 如果是最后一个，延续前缀用 "    ", 否则用 "│   "
            let extension_prefix = if is_last { "    " } else { "│   " };
            build_tree_recursive(&child, format!("{}{}", prefix, extension_prefix), lines)?;
        }
    }

    Ok(())
} 