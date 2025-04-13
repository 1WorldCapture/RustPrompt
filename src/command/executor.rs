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

// [ADDED] 定义一个函数，用于判断给定 Command 是否在指定模式下可用
fn is_command_valid_in_mode(cmd: &Command, mode: &ReplMode) -> bool {
    match mode {
        ReplMode::Manual => {
            match cmd {
                Command::Add(_) 
                | Command::Remove(_) 
                | Command::ShowContext
                | Command::Copy
                | Command::Reset
                | Command::Help
                | Command::Quit
                | Command::Mode(_) => true,

                // prompt 模式下特有的命令在 manual 模式无效
                Command::Prompt
                | Command::AppendPromptText(_)
                // [MODIFIED] Unknown 应该在任何模式下都"无效"（因为它不是一个已知命令，这里返回 false 来避免执行）
                | Command::Unknown(_) => false, 
            }
        }
        ReplMode::Prompt => {
            match cmd {
                // prompt 模式可用
                Command::Mode(_)
                | Command::Prompt
                | Command::ShowContext
                | Command::Copy
                | Command::Help
                | Command::Quit
                // prompt模式下 AppendPromptText 是自动添加行
                | Command::AppendPromptText(_) => true,

                // manual 模式才有的命令在 prompt 模式无效
                Command::Add(_)
                | Command::Remove(_)
                | Command::Reset
                // [MODIFIED] Unknown 在 prompt 模式也无效
                | Command::Unknown(_) => false, 
            }
        }
    }
}

pub async fn execute(cmd: Command, state: Arc<Mutex<AppState>>) -> Result<(), AppError> {
    let ignore_config = IgnoreConfig::default();

    // [ADDED] 先检查当前模式和命令的匹配性
    let current_mode = {
        let st = state.lock().unwrap();
        st.mode.clone()
    };

    // [MODIFIED] 对 Unknown 命令特殊处理，直接在 match 之前提示
    if let Command::Unknown(u) = &cmd {
        println!("未知命令: {}", u);
        return Ok(());
    }

    // [MODIFIED] 对其他命令进行有效性检查
    if !is_command_valid_in_mode(&cmd, &current_mode) {
        // 获取命令的简单名称用于打印
        let cmd_name = match &cmd {
             Command::Add(_) => "/add",
             Command::Remove(_) => "/remove",
             Command::ShowContext => "/context",
             Command::Copy => "/copy",
             Command::Reset => "/reset",
             Command::Help => "/help",
             Command::Quit => "/quit",
             Command::Mode(_) => "/mode",
             Command::Prompt => "/prompt",
             Command::AppendPromptText(_) => "(文本输入)",
             Command::Unknown(_) => "未知", // 理论上不会到这里
        };
        println!("(提示) 命令 {} 在 {:?} 模式不可用!", cmd_name, current_mode);
        return Ok(()); 
    }

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
            // [MODIFIED] 不同模式下只显示对应命令，并添加对齐的解释
            let st = state.lock().unwrap();
            let mode = st.mode.clone();
            drop(st); // Release the lock explicitly

            // [ADDED] 定义对齐宽度
            let width = 25;

            match mode {
                ReplMode::Manual => {
                    println!("可用命令(Manual模式):");
                    // [MODIFIED] 直接将参数传递给 println! 进行格式化
                    println!("  {:<width$} - {}", "/add <path>", "添加文件或目录到上下文", width=width);
                    println!("  {:<width$} - {}", "/remove <path>", "从上下文中移除文件或目录", width=width);
                    println!("  {:<width$} - {}", "/context", "显示当前上下文信息 (文件数, token数)", width=width);
                    println!("  {:<width$} - {}", "/copy", "将当前上下文(含项目树和提示词)复制到剪贴板", width=width);
                    println!("  {:<width$} - {}", "/reset", "清空所有上下文和提示词", width=width);
                    println!("  {:<width$} - {}", "/mode [manual|prompt]", "查看或切换模式", width=width);
                    println!("  {:<width$} - {}", "/help", "显示此帮助信息", width=width);
                    println!("  {:<width$} - {}", "/quit", "退出程序", width=width);
                }
                ReplMode::Prompt => {
                    println!("可用命令(Prompt模式):");
                    // [MODIFIED] 直接将参数传递给 println! 进行格式化
                    println!("  {:<width$} - {}", "/mode [manual|prompt]", "查看或切换模式", width=width);
                    println!("  {:<width$} - {}", "/prompt", "查看当前累积的提示词", width=width);
                    println!("  {:<width$} - {}", "/context", "显示当前上下文信息 (文件数, token数)", width=width);
                    println!("  {:<width$} - {}", "/copy", "将当前上下文(含项目树和提示词)复制到剪贴板", width=width);
                    println!("  {:<width$} - {}", "/help", "显示此帮助信息", width=width);
                    println!("  {:<width$} - {}", "/quit", "退出程序", width=width);
                    println!("\n在 prompt 模式下:");
                    println!("  直接输入内容(不以'/'开头)会被追加到提示词中。");
                }
            }
        }

        Command::Quit => {
            println!("(提示) 即将退出...");
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
                // This case should theoretically not be reached due to the check at the beginning
                eprintln!("内部错误：尝试在非 prompt 模式下追加提示词。");
            }
        }
        // [ADDED] Make sure all command variants are handled or explicitly ignored
        Command::Unknown(_) => { /* Already handled earlier */ }
    }

    Ok(())
} 