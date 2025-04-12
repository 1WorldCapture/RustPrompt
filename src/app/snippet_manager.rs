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
    /// 增量添加文件 snippet
    ///  - 先在锁外读取文件内容，生成 snippet
    ///  - 然后在锁内写入 partial_docs
    pub async fn add_files_snippet(
        state: Arc<Mutex<AppState>>,
        files: Vec<PathBuf>,
    ) -> Result<(), AppError> {
        // 1) 读取文件内容(在锁外, 避免阻塞 REPL)
        let mut new_snips = Vec::with_capacity(files.len());
        for f in &files { // Borrow files instead of consuming
            // 可以考虑 tokio::task::spawn_blocking，如果文件很多或很大
            let content = fs::read_to_string(f).await.unwrap_or_default();
            let snippet = generate_single_file_snippet(f, &content, 0);
            new_snips.push((f.clone(), snippet)); // Clone f here
        }

        // 2) 上锁: 将结果写入 partial_docs
        {
            let mut st = state.lock().unwrap();
            for (path, snip) in new_snips {
                st.partial_docs.insert(path, snip);
            }
        }

        Ok(())
    }

    /// 更新/重新生成项目树 snippet，并存入 partial_docs
    ///  - tree_builder 本身可能比较耗时, 可以考虑 spawn_blocking
    pub fn update_project_tree_snippet(
        state: Arc<Mutex<AppState>>,
        ignore_config: &IgnoreConfig,
    ) -> Result<(), AppError> {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        // 如果项目很大, generate_project_tree_string 可能耗时
        // 考虑优化: let tree_txt = tokio::task::spawn_blocking(...).await??;
        let tree_txt = generate_project_tree_string(&current_dir, ignore_config)
            .unwrap_or_else(|e| {
                log::error!("生成项目树失败: {:?}", e);
                "".to_string()
            });

        let snippet = generate_single_file_snippet(Path::new(PROJECT_TREE_VIRTUAL_PATH), &tree_txt, 0);

        {
            let mut st = state.lock().unwrap();
            st.partial_docs.insert(PathBuf::from(PROJECT_TREE_VIRTUAL_PATH), snippet);
        }

        Ok(())
    }

    /// 重建合并 + 计算token
    ///  - 在锁内进行，合并和token计算本身不算大IO
    pub fn rebuild_and_recalc(state: Arc<Mutex<AppState>>) -> Result<(), AppError> {
        let mut st = state.lock().unwrap();
        let merged = merge_all_snippets(&st.partial_docs);
        let tokens = calculate_tokens_in_string(&merged)?;
        st.cached_xml = merged;
        st.token_count = tokens;
        Ok(())
    }

    /// 全量刷新: 清空除项目树外的 snippet -> 重新生成 -> 更新树 -> 计算 token
    ///  - 在锁外进行文件IO
    pub async fn full_refresh(
        state: Arc<Mutex<AppState>>,
        all_paths: Vec<PathBuf>,
        ignore_config: &IgnoreConfig,
    ) -> Result<(), AppError> {
        // 1) 先清空 old snippet (在锁内，快速操作)
        {
            let mut st = state.lock().unwrap();
            st.partial_docs.clear(); // 清空所有真实文件 snippet
            // 暂时不写回 tree snippet，等文件IO完成后再统一处理
        }

        // 2) 读取文件IO (锁外)
        let mut new_snips = Vec::with_capacity(all_paths.len());
        for f in &all_paths { // Borrow all_paths
            let content = fs::read_to_string(f).await.unwrap_or_default();
            let snippet = generate_single_file_snippet(f, &content, 0);
            new_snips.push((f.clone(), snippet)); // Clone path here
        }

        // 3) 更新项目树 (同样可能耗时，锁外执行，但目前是同步函数)
        //    为了简化，先在锁外生成树文本和 snippet
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let tree_txt = generate_project_tree_string(&current_dir, ignore_config)
            .unwrap_or_else(|e| {
                log::error!("生成项目树失败: {:?}", e);
                "".to_string()
            });
        let tree_snippet = generate_single_file_snippet(Path::new(PROJECT_TREE_VIRTUAL_PATH), &tree_txt, 0);

        // 4) 上锁一次性写回所有 snippets (包括新的项目树)
        {
            let mut st = state.lock().unwrap();
            for (path, snip) in new_snips {
                st.partial_docs.insert(path, snip);
            }
            // 写入新的或恢复的树 snippet
            st.partial_docs.insert(PathBuf::from(PROJECT_TREE_VIRTUAL_PATH), tree_snippet);
        }

        // 5) rebuild & recalc (锁内)
        Self::rebuild_and_recalc(state)?;

        Ok(())
    }
} 