// src/core/xml.rs
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use crate::app::state::PROJECT_TREE_VIRTUAL_PATH;

/// 生成单个文件的 snippet (不包含 <documents> 根标签)
pub fn generate_single_file_snippet(
    path: &Path,
    content: &str,
    index: usize, // 临时索引，在 merge 时会被替换
) -> String {
    let path_str = path.to_string_lossy();
    // 使用原始字符串简化转义
    format!(
        r#"<document index="{idx}">
<source>{src}</source>
<document_content>
{body}
</document_content>
</document>"#,
        idx = index,
        src = path_str,
        body = content,
    )
}

/// 合并 partial_docs 里的 snippet，生成完整的 <documents>... XML。
/// 其中:
///   - __PROJECT_TREE__ 对应的 snippet 被视为 index=1
///   - 其余文档按路径排序后，从 index=2 开始
pub fn merge_all_snippets(partial_docs: &HashMap<PathBuf, String>) -> String {
    // 1) 找到项目树 snippet (若不存在则为空)
    let tree_key = PathBuf::from(PROJECT_TREE_VIRTUAL_PATH);
    let maybe_tree_snip = partial_docs.get(&tree_key);

    // 2) 收集其它文件 snippet
    let mut real_files: Vec<(&PathBuf, &String)> = partial_docs
        .iter()
        .filter(|(k, _)| **k != tree_key) // 过滤掉项目树
        .collect();

    // 以路径字符串排序，确保一致性
    real_files.sort_by(|a, b| a.0.cmp(&b.0));

    // 3) 开始拼装
    let mut result = String::new();
    result.push_str("<documents>\n");

    // 3.1) 若有项目树 snippet
    if let Some(tree_snip) = maybe_tree_snip {
        // 强行把它当成 index=1
        let updated = replace_doc_index(tree_snip, 1);
        result.push_str(&updated);
        result.push('\n'); // 每个 snippet 后加换行
    }

    // 3.2) 依次给真实文件 snippet 分配 index=2,3,...
    let mut doc_index = 2;
    for (_, snip) in real_files {
        let updated = replace_doc_index(snip, doc_index);
        result.push_str(&updated);
        result.push('\n'); // 每个 snippet 后加换行
        doc_index += 1;
    }

    result.push_str("</documents>");
    result
}

/// 将 snippet 里的 index="X" 替换为 index="new_index"
/// 使用字符串查找和替换，避免引入 XML 解析库的复杂性
fn replace_doc_index(original: &str, new_index: usize) -> String {
    let mut result = original.to_string();
    let pattern_start = r#"index=""#;
    if let Some(pos) = result.find(pattern_start) {
        let start_idx = pos + pattern_start.len();
        // 从 start_idx 开始查找第一个 "
        if let Some(end_quote) = result[start_idx..].find('"') {
            let end_idx = start_idx + end_quote;
            // 构造新的 index 字符串
            let new_str = new_index.to_string();
            // 替换范围
            result.replace_range(start_idx..end_idx, &new_str);
        }
    }
    result
}

// Removed unused generate_xml function
/*
pub async fn generate_xml(paths: &[PathBuf]) -> Result<String, AppError> {
    let mut partial_docs = HashMap::new();
    let ignore_config = IgnoreConfig::default();
    
    // 1. 生成项目树 snippet
    let current_dir = std::env::current_dir()
        .map_err(|e| AppError::General(anyhow!("无法获取当前目录: {:?}", e)))?;
    let tree_txt = generate_project_tree_string(&current_dir, &ignore_config)?;
    let tree_snippet = generate_single_file_snippet(
        Path::new(PROJECT_TREE_VIRTUAL_PATH),
        &tree_txt,
        0, // 临时index，merge时会变成1
    );
    partial_docs.insert(PathBuf::from(PROJECT_TREE_VIRTUAL_PATH), tree_snippet);

    // 2. 生成真实文件 snippets
    for path in paths {
        match fs::read_to_string(path).await {
            Ok(content) => {
                let snippet = generate_single_file_snippet(path, &content, 0);
                partial_docs.insert(path.clone(), snippet);
            }
            Err(err) => {
                warn!("读取 {:?} 失败: {:?}", path, err);
            }
        }
    }

    // 3. 合并所有 snippets
    let merged = merge_all_snippets(&partial_docs);
    Ok(merged)
}
*/ 