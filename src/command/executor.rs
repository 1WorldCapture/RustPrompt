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
    repl::engine::ReplEngine,
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
                | Command::Mode(_)
                | Command::ResetPrompt  // 允许在 Manual 模式下使用
                | Command::Prompt  // 允许在 Manual 模式下使用
                => true,

                Command::AppendPromptText(_)
                | Command::Unknown(_) => false,
            }
        }
        ReplMode::Prompt => {
            match cmd {
                Command::Mode(_)
                | Command::Prompt
                | Command::ShowContext
                | Command::Copy
                | Command::Help
                | Command::Quit
                | Command::AppendPromptText(_)
                | Command::ResetPrompt  // 允许在 Prompt 模式下使用
                => true,

                Command::Add(_)
                | Command::Remove(_)
                | Command::Reset
                | Command::Unknown(_) => false,
            }
        }
    }
}

pub async fn execute(
    cmd: Command, 
    state: Arc<Mutex<AppState>>,
    engine: &mut ReplEngine,
) -> Result<(), AppError> {
    let ignore_config = IgnoreConfig::default();

    // [ADDED] Check the compatibility between current mode and command
    let current_mode = {
        let st = state.lock().unwrap();
        st.mode.clone()
    };

    // [MODIFIED] Handle Unknown command specially, prompt before match
    if let Command::Unknown(u) = &cmd {
        println!("Unknown command: {}", u);
        return Ok(());
    }

    // [MODIFIED] Check validity of other commands
    if !is_command_valid_in_mode(&cmd, &current_mode) {
        // Get simple command name for printing
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
             Command::AppendPromptText(_) => "(text input)",
             Command::ResetPrompt => "/resetprompt",
             Command::Unknown(_) => "unknown",
        };
        println!("(Note) Command {} is not available in {:?} mode!", cmd_name, current_mode);
        return Ok(()); 
    }

    match cmd {
        Command::Add(path) => {
            info!("Executing /add: {:?}", path);

            let scanned = files_scanner::scan_dir(&path, &ignore_config).await?;
            info!("  -> Scanned {} files", scanned.len());

            let num_added = {
                let mut st = state.lock().unwrap();
                let init_count = st.selected_paths.len();
                for f in &scanned {
                    st.selected_paths.insert(f.clone());
                }
                let final_count = st.selected_paths.len();
                st.file_count = final_count;
                info!("  -> selected_paths increased from {} to {}", init_count, final_count);
                final_count - init_count
            };

            if num_added > 0 || scanned.is_empty() {
                SnippetManager::add_files_snippet(state.clone(), scanned).await?;
                SnippetManager::update_project_tree_snippet(state.clone(), &ignore_config)?;
                SnippetManager::rebuild_and_recalc(state.clone())?;
            } else {
                info!("  -> No new files added, skipping snippet update");
            }
        }

        Command::Remove(path) => {
            info!("Executing /remove: {:?}", path);

            let scanned = files_scanner::scan_dir(&path, &ignore_config).await?;
            info!("  -> Scanned {} files (to be removed)", scanned.len());

            let num_removed = {
                let mut st = state.lock().unwrap();
                let init_count = st.selected_paths.len();
                for f in &scanned {
                    st.selected_paths.remove(f);
                    st.partial_docs.remove(f);
                }
                let final_count = st.selected_paths.len();
                st.file_count = final_count;
                info!("  -> selected_paths decreased from {} to {}", init_count, final_count);
                init_count - final_count
            };

            if num_removed > 0 {
                SnippetManager::update_project_tree_snippet(state.clone(), &ignore_config)?;
                SnippetManager::rebuild_and_recalc(state.clone())?;
            } else {
                info!("  -> No files removed, skipping snippet update");
            }
        }

        Command::ShowContext => {
            let st = state.lock().unwrap();
            println!("Current file_count={}, token_count={}", st.file_count, st.token_count);
            println!("Selected files:");
            for p in &st.selected_paths {
                println!(" - {:?}", p);
            }
        }

        Command::Copy => {
            info!("Executing /copy (full refresh)");

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
                Ok(_) => println!("(Note) Content (including project tree + instruction) has been copied to clipboard!"),
                Err(e) => eprintln!("Failed to copy to clipboard: {:?}", e),
            }
        }

        Command::Reset => {
            info!("Executing /reset");
            let mut st = state.lock().unwrap();
            st.selected_paths.clear();
            st.file_count = 0;
            st.token_count = 0;
            st.partial_docs.clear();
            st.cached_xml.clear();
            st.prompt_text.clear();

            info!("  -> All context cleared (files, partial_docs, token_count, prompt_text)");
        }

        Command::Help => {
            // [MODIFIED] Show commands for different modes with aligned descriptions
            let st = state.lock().unwrap();
            let mode = st.mode.clone();
            drop(st); // Release the lock explicitly

            // [ADDED] Define alignment width
            let width = 25;

            match mode {
                ReplMode::Manual => {
                    println!("Available commands (Manual mode):");
                    println!("  {:<width$} - {}", "/add <path>", "Add files or directories to context", width=width);
                    println!("  {:<width$} - {}", "/remove <path>", "Remove files or directories from context", width=width);
                    println!("  {:<width$} - {}", "/context", "Show current context info (file count, token count)", width=width);
                    println!("  {:<width$} - {}", "/copy", "Copy current context (with project tree and prompt) to clipboard", width=width);
                    println!("  {:<width$} - {}", "/reset", "Clear all context and prompt", width=width);
                    println!("  {:<width$} - {}", "/mode [manual|prompt]", "View or switch modes", width=width);
                    println!("  {:<width$} - {}", "/help", "Show this help message", width=width);
                    println!("  {:<width$} - {}", "/quit", "Exit program", width=width);
                }
                ReplMode::Prompt => {
                    println!("Available commands (Prompt mode):");
                    println!("  {:<width$} - {}", "/mode [manual|prompt]", "View or switch modes", width=width);
                    println!("  {:<width$} - {}", "/prompt", "View current accumulated prompt", width=width);
                    println!("  {:<width$} - {}", "/context", "Show current context info (file count, token count)", width=width);
                    println!("  {:<width$} - {}", "/copy", "Copy current context (with project tree and prompt) to clipboard", width=width);
                    println!("  {:<width$} - {}", "/help", "Show this help message", width=width);
                    println!("  {:<width$} - {}", "/quit", "Exit program", width=width);
                    println!("\nIn prompt mode:");
                    println!("  Direct input (not starting with '/') will be appended to the prompt.");
                }
            }
        }

        Command::Quit => {
            println!("(Note) Exiting...");
        }

        Command::Mode(opt) => {
            let mut st = state.lock().unwrap();
            match opt {
                None => {
                    match st.mode {
                        ReplMode::Manual => println!("Current mode: manual"),
                        ReplMode::Prompt => println!("Current mode: prompt"),
                    }
                }
                Some(m) => {
                    let mode_str = m.to_lowercase();
                    if mode_str == "manual" {
                        st.mode = ReplMode::Manual;
                        println!("Switched to manual mode");
                    } else if mode_str == "prompt" {
                        st.mode = ReplMode::Prompt;
                        println!("Switched to prompt mode");
                    } else {
                        println!("Unknown mode: {} (available: manual, prompt)", m);
                    }
                }
            }
        }

        Command::Prompt => {
            // If currently in Manual mode, automatically switch to Prompt mode
            {
                let mut st = state.lock().unwrap();
                if st.mode == ReplMode::Manual {
                    println!("(Note) Currently in manual mode, automatically switching to prompt mode...");
                    st.mode = ReplMode::Prompt;
                }
            }
            // Enter multiline edit mode
            engine.enter_multiline_mode()?;
            println!("(Note) Entering multiline edit mode. Type :submit and press Enter to finish editing.");
        }

        Command::ResetPrompt => {
            let mut st = state.lock().unwrap();
            st.prompt_text.clear();
            println!("(Note) Prompt cache has been cleared.");
        }

        Command::AppendPromptText(line) => {
            let mut st = state.lock().unwrap();
            if st.mode == ReplMode::Prompt {
                if !st.prompt_text.is_empty() {
                    st.prompt_text.push('\n');
                }
                st.prompt_text.push_str(&line);
                println!("(Note) Added to prompt");
            } else {
                eprintln!("Internal error: Attempting to append prompt text in non-prompt mode.");
            }
        }
        // [ADDED] Make sure all command variants are handled or explicitly ignored
        Command::Unknown(_) => { /* Already handled earlier */ }
    }

    Ok(())
} 