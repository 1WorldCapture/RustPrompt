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
            let scanned_files = files_scanner::scan_dir(&path, &ignore_config).await?;
            info!("  -> 扫描到 {} 个文件 (非忽略)", scanned_files.len());

            {
                let mut st = state.lock().unwrap();
                let initial_count = st.selected_paths.len();
                for f in &scanned_files {
                    st.selected_paths.insert(f.clone());
                }
                st.file_count = st.selected_paths.len();
                info!("  -> selected_paths 从 {} 增加到 {} 个", initial_count, st.file_count);
            }

            SnippetManager::add_files_snippet(state.clone(), scanned_files).await?;

            SnippetManager::update_project_tree_snippet(state.clone(), &ignore_config)?;

            SnippetManager::rebuild_and_recalc(state.clone())?;
        }

        Command::Remove(path) => {
            info!("执行 /remove: {:?}", path);
            let scanned_files = files_scanner::scan_dir(&path, &ignore_config).await?;
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

            SnippetManager::update_project_tree_snippet(state.clone(), &ignore_config)?;

            SnippetManager::rebuild_and_recalc(state.clone())?;
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

            let all_paths: Vec<PathBuf> = {
                let st = state.lock().unwrap();
                st.selected_paths.iter().cloned().collect()
            };

            SnippetManager::full_refresh(state.clone(), all_paths, &ignore_config).await?;

            let xml_to_copy = {
                let st = state.lock().unwrap();
                st.cached_xml.clone()
            };
            if let Err(e) = clipboard::copy_to_clipboard(&xml_to_copy) {
                eprintln!("复制到剪贴板失败: {}", e);
            } else {
                println!("已将选中的内容(含项目树)复制到剪贴板!");
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