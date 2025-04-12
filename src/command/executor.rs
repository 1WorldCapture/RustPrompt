use std::sync::{Arc, Mutex};
use std::path::PathBuf;

use log::info;
use anyhow::Result;

use crate::{
    app::state::AppState,
    app::snippet_manager::SnippetManager,
    command::definition::Command,
    core::{files_scanner, ignore_rules::IgnoreConfig, clipboard},
    error::AppError,
};

pub async fn execute(cmd: Command, state: Arc<Mutex<AppState>>) -> Result<(), AppError> {
    let ignore_config = IgnoreConfig::default();

    match cmd {
        Command::Add(path) => {
            info!("执行 /add: {:?}", path);

            let scanned = files_scanner::scan_dir(&path, &ignore_config).await?;
            info!("  -> 扫描到 {} 个文件", scanned.len());

            let num_added = {
                let mut st = state.lock().unwrap();
                let init_count = st.selected_paths.len();
                for f in &scanned {
                    st.selected_paths.insert(f.clone());
                }
                let final_count = st.selected_paths.len();
                st.file_count = final_count;
                info!("  -> selected_paths 从 {} 增到 {}", init_count, final_count);
                final_count - init_count
            };

            if num_added > 0 || scanned.is_empty() {
                SnippetManager::add_files_snippet(state.clone(), scanned).await?;
                SnippetManager::update_project_tree_snippet(state.clone(), &ignore_config)?;
                SnippetManager::rebuild_and_recalc(state.clone())?;
            } else {
                info!("  -> 没有新文件被添加，跳过 snippet 更新");
            }
        }

        Command::Remove(path) => {
            info!("执行 /remove: {:?}", path);

            let scanned = files_scanner::scan_dir(&path, &ignore_config).await?;
            info!("  -> 扫描到 {} 个文件 (待移除)", scanned.len());

            let num_removed = {
                let mut st = state.lock().unwrap();
                let init_count = st.selected_paths.len();
                for f in &scanned {
                    st.selected_paths.remove(f);
                    st.partial_docs.remove(f);
                }
                let final_count = st.selected_paths.len();
                st.file_count = final_count;
                info!("  -> selected_paths 从 {} 减到 {}", init_count, final_count);
                init_count - final_count
            };

            if num_removed > 0 {
                SnippetManager::update_project_tree_snippet(state.clone(), &ignore_config)?;
                SnippetManager::rebuild_and_recalc(state.clone())?;
            } else {
                info!("  -> 没有文件被移除，跳过 snippet 更新");
            }
        }

        Command::ShowContext => {
            let st = state.lock().unwrap();
            println!("当前 file_count={}, token_count={}", st.file_count, st.token_count);
            println!("已选文件列表:");
            for p in &st.selected_paths {
                println!(" - {:?}", p);
            }
        }

        Command::Copy => {
            info!("执行 /copy (全量刷新)");

            let paths: Vec<PathBuf> = {
                let st = state.lock().unwrap();
                st.selected_paths.iter().cloned().collect()
            };

            SnippetManager::full_refresh(state.clone(), paths, &ignore_config).await?;

            let xml = {
                let st = state.lock().unwrap();
                st.cached_xml.clone()
            };

            match clipboard::copy_to_clipboard(&xml) {
                Ok(_) => println!("(提示) 已将选中的内容(含项目树)复制到剪贴板!"),
                Err(e) => eprintln!("复制到剪贴板失败: {:?}", e),
            }
        }

        Command::Reset => {
            info!("执行 /reset");
            let mut st = state.lock().unwrap();
            st.selected_paths.clear();
            st.file_count = 0;
            st.token_count = 0;
            st.partial_docs.clear();
            st.cached_xml.clear();

            info!("  -> 已清空所有上下文 (files, partial_docs, token_count)");
        }

        Command::Help => {
            println!("可用命令:");
            println!("  /add <path>");
            println!("  /remove <path>");
            println!("  /context");
            println!("  /copy");
            println!("  /reset");
            println!("  /help");
            println!("  /quit");
        }

        Command::Quit => {
            println!("(提示) 即将退出...");
        }

        Command::Unknown(u) => {
            println!("未知命令: {}", u);
        }
    }

    Ok(())
} 