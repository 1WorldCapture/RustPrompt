use std::sync::{Arc, Mutex};
use std::path::PathBuf;

use log::info;
use anyhow::Result;

use crate::{
    app::state::{AppState, ReplMode},
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

            let xml_to_copy = {
                let mut st = state.lock().unwrap();
                let mut final_xml = st.cached_xml.clone();

                if !st.prompt_text.is_empty() {
                    let instruction_tag = format!("\n<instruction>\n{}\n</instruction>", st.prompt_text);

                    if let Some(idx) = final_xml.rfind("</documents>") {
                        final_xml.insert_str(idx, &instruction_tag);
                    } else {
                        final_xml.push_str(&instruction_tag);
                        final_xml.push_str("\n</documents>");
                    }
                    st.cached_xml = final_xml.clone();
                }
                final_xml
            };

            match clipboard::copy_to_clipboard(&xml_to_copy) {
                Ok(_) => println!("(提示) 已将选中的内容(含项目树 + instruction)复制到剪贴板!"),
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
            st.prompt_text.clear();

            info!("  -> 已清空所有上下文 (files, partial_docs, token_count, prompt_text)");
        }

        Command::Help => {
            println!("可用命令:");
            println!("  /add <path>");
            println!("  /remove <path>");
            println!("  /context");
            println!("  /copy");
            println!("  /reset");
            println!("  /mode [manual|prompt]  - 查看或切换模式");
            println!("  /prompt              - 查看当前提示词 (prompt模式下生效)");
            println!("  /help");
            println!("  /quit");
            println!("\n在 prompt 模式下:");
            println!("  直接输入内容会被追加到提示词中。");
        }

        Command::Quit => {
            println!("(提示) 即将退出...");
        }

        Command::Unknown(u) => {
            println!("未知命令: {}", u);
        }

        Command::Mode(opt) => {
            let mut st = state.lock().unwrap();
            match opt {
                None => {
                    match st.mode {
                        ReplMode::Manual => println!("当前模式: manual"),
                        ReplMode::Prompt => println!("当前模式: prompt"),
                    }
                }
                Some(m) => {
                    let mode_str = m.to_lowercase();
                    if mode_str == "manual" {
                        st.mode = ReplMode::Manual;
                        println!("已切换到 manual 模式");
                    } else if mode_str == "prompt" {
                        st.mode = ReplMode::Prompt;
                        println!("已切换到 prompt 模式");
                    } else {
                        println!("未知模式: {} (可用: manual, prompt)", m);
                    }
                }
            }
        }

        Command::Prompt => {
            let st = state.lock().unwrap();
            if st.mode == ReplMode::Prompt {
                println!("(提示) 当前 prompt_text:\n{}", st.prompt_text);
            } else {
                println!("(提示) /prompt 命令仅在 prompt 模式下查看提示词。当前为 manual 模式。");
            }
        }

        Command::AppendPromptText(line) => {
            let mut st = state.lock().unwrap();
            if st.mode == ReplMode::Prompt {
                if !st.prompt_text.is_empty() {
                    st.prompt_text.push('\n');
                }
                st.prompt_text.push_str(&line);
                println!("(提示) 已添加到提示词");
            } else {
                eprintln!("内部错误：尝试在非 prompt 模式下追加提示词。");
            }
        }
    }

    Ok(())
} 