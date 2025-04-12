use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::fs;

use crate::{
    app::state::{AppState, PROJECT_TREE_VIRTUAL_PATH},
    core::{
        tokenizer::calculate_tokens_in_string,
        xml::{generate_single_file_snippet, merge_all_snippets},
        tree_builder::generate_project_tree_string, // 使用 tree_builder
        ignore_rules::IgnoreConfig,
    },
    error::AppError,
};

/// 提供对 snippet 的公共操作，如增量更新、全量刷新、更新项目树、重建合并等。
pub struct SnippetManager;

impl SnippetManager {
    /// 增量添加文件 snippet (读取IO后写入 AppState.partial_docs)
    pub async fn add_files_snippet(
        state: Arc<Mutex<AppState>>,
        files: Vec<PathBuf>, // 修改为 Vec<PathBuf> 以匹配 executor 调用
    ) -> Result<(), AppError> {
        // 1) 读取文件内容(锁外)
        let mut new_snips = Vec::with_capacity(files.len());
        for f in files {
            let content = fs::read_to_string(&f).await.unwrap_or_default();
            // 暂时给 snippet 用 index=0, 后续 merge_all_snippets 会统一替换
            let snippet = generate_single_file_snippet(&f, &content, 0);
            new_snips.push((f, snippet));
        }

        // 2) 上锁，写入 partial_docs
        {
            let mut st = state.lock().unwrap();
            for (path, snip) in new_snips {
                st.partial_docs.insert(path, snip);
            }
        }

        Ok(())
    }

    /// 更新(或重新生成)项目树 snippet，并放入 partial_docs
    pub fn update_project_tree_snippet(
        state: Arc<Mutex<AppState>>,
        ignore_config: &IgnoreConfig,
    ) -> Result<(), AppError> {
        // 1) 生成树字符串(不需锁)
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")); // Handle error better
        let tree_txt = generate_project_tree_string(&current_dir, ignore_config)
            .unwrap_or_else(|e| {
                log::error!("生成项目树失败: {:?}", e); // Log error
                "".to_string()
            });

        // 2) 生成 snippet
        let snippet = generate_single_file_snippet(Path::new(PROJECT_TREE_VIRTUAL_PATH), &tree_txt, 0);

        // 3) 上锁写入 partial_docs
        {
            let mut st = state.lock().unwrap();
            st.partial_docs.insert(PathBuf::from(PROJECT_TREE_VIRTUAL_PATH), snippet);
        }

        Ok(())
    }

    /// 重建合并 + 计算token
    pub fn rebuild_and_recalc(state: Arc<Mutex<AppState>>) -> Result<(), AppError> {
        let mut st = state.lock().unwrap();
        let merged = merge_all_snippets(&st.partial_docs);
        let tokens = calculate_tokens_in_string(&merged)?; // 计算前先合并
        st.cached_xml = merged; // 在计算后赋值
        st.token_count = tokens;
        Ok(())
    }

    /// 全量刷新：清空除项目树外的 snippet，重新生成所有 + 更新项目树 + 重建合并
    /// 注意：这里是一次性函数，用在 /copy 等命令
    pub async fn full_refresh(
        state: Arc<Mutex<AppState>>,
        all_paths: Vec<PathBuf>,
        ignore_config: &IgnoreConfig,
    ) -> Result<(), AppError> {
        // 1) 保留原先 project_tree 的 snippet, 其余清空
        {
            let mut st = state.lock().unwrap();
            let tree_snip = st.partial_docs.remove(&PathBuf::from(PROJECT_TREE_VIRTUAL_PATH));
            st.partial_docs.clear(); // 清空所有，包括旧树（如果存在）
            if let Some(snip) = tree_snip {
                // 重新插入，确保总是有一个树的位置（即使是旧的，后面会更新）
                st.partial_docs.insert(PathBuf::from(PROJECT_TREE_VIRTUAL_PATH), snip);
            }
        }

        // 2) 重新生成所有文件 snippet
        SnippetManager::add_files_snippet(state.clone(), all_paths).await?;

        // 3) 更新项目树 (确保总是执行，即使之前没有树)
        SnippetManager::update_project_tree_snippet(state.clone(), ignore_config)?;

        // 4) rebuild & recalc
        SnippetManager::rebuild_and_recalc(state)?;

        Ok(())
    }
} 