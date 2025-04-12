use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};

use log::info;
use anyhow::Result;

use crate::{
    app::state::{AppState, PROJECT_TREE_VIRTUAL_PATH},
    command::definition::Command,
    core::{
        files,
        tokenizer::calculate_tokens_in_string,
        xml::{generate_single_file_snippet, merge_all_snippets},
        tree::generate_project_tree_string,
        clipboard,
        ignore_rules::IgnoreConfig,
    },
    error::AppError,
};

pub async fn execute(cmd: Command, state: Arc<Mutex<AppState>>) -> Result<(), AppError> {
    let ignore_config = IgnoreConfig::default();

    match cmd {
        Command::Add(path) => {
            info!("执行 /add: {:?}", path);
            let scanned_files = files::scan_dir(&path, &ignore_config).await?;
            info!("  -> 扫描到 {} 个文件", scanned_files.len());

            {
                let mut st = state.lock().unwrap();
                let initial_count = st.selected_paths.len();
                for f in &scanned_files {
                    st.selected_paths.insert(f.clone());
                }
                st.file_count = st.selected_paths.len();
                info!("  -> selected_paths 从 {} 增加到 {} 个", initial_count, st.file_count);
            }

            // (1) 读取真实文件内容并增量更新 snippet
            incrementally_update_real_files(state.clone(), &scanned_files).await?;

            // (2) 更新项目树 snippet (传递 ignore_config)
            update_project_tree_snippet(state.clone(), &ignore_config).await?;

            // (3) 重新合并 & 计算token
            rebuild_and_recalc(state.clone())?;
        }

        Command::Remove(path) => {
            info!("执行 /remove: {:?}", path);
            let scanned_files = files::scan_dir(&path, &ignore_config).await?;
            info!("  -> 扫描到 {} 个文件 (待移除)", scanned_files.len());

            {
                let mut st = state.lock().unwrap();
                let initial_count = st.selected_paths.len();
                for f in &scanned_files {
                    st.selected_paths.remove(f);
                    st.partial_docs.remove(f);
                }
                st.file_count = st.selected_paths.len();
                info!("  -> selected_paths 从 {} 减少到 {} 个", initial_count, st.file_count);
            }

            // (2) 更新项目树 snippet (传递 ignore_config)
            update_project_tree_snippet(state.clone(), &ignore_config).await?;

            // (3) 重新合并 & 计算token
            rebuild_and_recalc(state.clone())?;
        }

        Command::ShowContext => {
            let st = state.lock().unwrap();
            println!("当前已选文件数量: {}", st.file_count);
            println!("当前 token_count: {}", st.token_count);
            println!("已选文件列表:");
            for p in &st.selected_paths {
                println!(" - {:?}", p);
            }
        }

        Command::Copy => {
            info!("执行 /copy (全量刷新)");

            // 1) 对所有选中的真实文件重新读取内容
            let all_paths: Vec<PathBuf> = {
                let st = state.lock().unwrap();
                st.selected_paths.iter().cloned().collect()
            };

            {
                let mut st = state.lock().unwrap();
                // 清空 partial_docs 里的真实文件相关 snippet
                let tree_snip = st.partial_docs.remove(&PathBuf::from(PROJECT_TREE_VIRTUAL_PATH));
                st.partial_docs.clear();
                if let Some(tree) = tree_snip {
                    st.partial_docs.insert(PathBuf::from(PROJECT_TREE_VIRTUAL_PATH), tree);
                }
            }

            // 2) 重新生成真实文件 snippet
            incrementally_update_real_files(state.clone(), &all_paths).await?;
            // 3) 重新生成/更新项目树 snippet (传递 ignore_config)
            update_project_tree_snippet(state.clone(), &ignore_config).await?;
            // 4) rebuild & recalc
            rebuild_and_recalc(state.clone())?;

            // 5) 拷贝到剪贴板
            let xml_to_copy = {
                let st = state.lock().unwrap();
                st.cached_xml.clone()
            };
            match clipboard::copy_to_clipboard(&xml_to_copy) {
                Ok(_) => {
                    println!("已将选中的内容(含项目树)复制到剪贴板!");
                }
                Err(e) => {
                    eprintln!("复制到剪贴板失败: {}", e);
                }
            }
        }

        Command::Help => {
            println!("可用命令：");
            println!("  /add <path>");
            println!("  /remove <path>");
            println!("  /context");
            println!("  /copy");
            println!("  /help");
            println!("  /quit");
        }

        Command::Quit => {
            println!("收到 /quit 命令，即将退出。");
        }

        Command::Unknown(s) => {
            println!("未知命令: {}", s);
        }
    }

    Ok(())
}

/// 只负责「读取新文件内容 -> 生成 snippet -> 放入 partial_docs」
/// 并不做合并和token更新
async fn incrementally_update_real_files(
    state: Arc<Mutex<AppState>>,
    files: &[PathBuf],
) -> Result<(), AppError> {
    for f in files {
        let content = tokio::fs::read_to_string(f).await.unwrap_or_default();
        // 暂时给 snippet 用 index=0, 后续 merge_all_snippets 会统一替换
        let snippet = generate_single_file_snippet(f, &content, 0);
        let mut st = state.lock().unwrap();
        st.partial_docs.insert(f.clone(), snippet);
    }
    Ok(())
}

/// 生成/刷新 项目树 snippet，并存入 partial_docs (现在需要 ignore_config)
async fn update_project_tree_snippet(
    state: Arc<Mutex<AppState>>,
    ignore_config: &IgnoreConfig
) -> Result<(), AppError> {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let tree_txt = generate_project_tree_string(&current_dir, ignore_config).unwrap_or_default();
    let snippet = generate_single_file_snippet(
        Path::new(PROJECT_TREE_VIRTUAL_PATH),
        &tree_txt,
        0, // index=0, merge时再替换成1
    );
    {
        let mut st = state.lock().unwrap();
        st.partial_docs.insert(PathBuf::from(PROJECT_TREE_VIRTUAL_PATH), snippet);
    }
    Ok(())
}

/// 合并 partial_docs -> cached_xml, 并计算 token
fn rebuild_and_recalc(state: Arc<Mutex<AppState>>) -> Result<(), AppError> {
    let mut st = state.lock().unwrap();
    let merged = merge_all_snippets(&st.partial_docs);
    st.cached_xml = merged;
    let tokens = calculate_tokens_in_string(&st.cached_xml)?;
    st.token_count = tokens;
    Ok(())
} 