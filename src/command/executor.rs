use std::sync::{Arc, Mutex};
use std::path::PathBuf;

use anyhow::Result;
use log::info; // 用于记录状态更新

use crate::{app::state::AppState, command::definition::Command, core::{files, tokenizer, xml, clipboard}, error::AppError};

pub async fn execute(cmd: Command, state: Arc<Mutex<AppState>>) -> Result<(), AppError> {
    match cmd {
        Command::Add(path) => {
            // 1. 扫描得到所有文件
            info!("执行 /add: {:?}", path);
            let scanned_files = files::scan_dir(&path).await?;
            info!("  -> 扫描到 {} 个文件", scanned_files.len());

            // 2. 更新 AppState: 把新扫描到的文件加入 selected_paths
            {
                let mut st = state.lock().unwrap();
                let initial_count = st.selected_paths.len();
                for f in scanned_files.iter() {
                    st.selected_paths.insert(f.clone());
                }
                info!("  -> AppState.selected_paths 从 {} 增加到 {} 个", initial_count, st.selected_paths.len());
            }

            // 3. 重新计算 file_count & token_count
            recalc_file_and_tokens(state.clone()).await?;
        }

        Command::Remove(path) => {
            // 1. 扫描得到要移除的文件列表
            info!("执行 /remove: {:?}", path);
            let scanned_files = files::scan_dir(&path).await?;
            info!("  -> 扫描到 {} 个文件 (待移除)", scanned_files.len());

            // 2. 更新 AppState: 从 selected_paths 中移除这些文件
            {
                let mut st = state.lock().unwrap();
                let initial_count = st.selected_paths.len();
                for f in scanned_files.iter() {
                    st.selected_paths.remove(f);
                }
                 info!("  -> AppState.selected_paths 从 {} 减少到 {} 个", initial_count, st.selected_paths.len());
            }

            // 3. 重新计算 file_count & token_count
            recalc_file_and_tokens(state.clone()).await?;
        }

        Command::ShowContext => {
            println!("执行 /context. 已选文件如下：");
            let st = state.lock().unwrap();
            if st.selected_paths.is_empty() {
                println!("  (当前未选中任何文件)");
            } else {
                for p in &st.selected_paths {
                    println!(" - {:?}", p);
                }
            }
            println!("当前 file_count = {}, token_count = {}", st.file_count, st.token_count);
        }

        Command::Copy => {
            // 核心实现：读取所有选中文件 => 生成XML => 复制到剪贴板
            info!("执行 /copy");

            // 1. 收集当前选中文件 (锁 state)
            let all_files: Vec<PathBuf> = {
                let st = state.lock().unwrap();
                if st.selected_paths.is_empty() {
                    println!("(提示) 当前未选中文件，无法复制空XML。");
                    return Ok(());
                }
                st.selected_paths.iter().cloned().collect()
            };

            // 2. 生成 XML (异步)
            info!("  -> 开始生成包含 {} 个文件的 XML...", all_files.len());
            let xml_str = xml::generate_xml(&all_files).await?;
            info!("  -> XML 生成完毕，长度: {}", xml_str.len());

            // 3. 复制到剪贴板 (同步API，可能快)
            info!("  -> 尝试复制到剪贴板...");
            match clipboard::copy_to_clipboard(&xml_str) {
                Ok(_) => {
                    println!("已将选中文件的XML复制到剪贴板。");
                    info!("  -> 复制成功!");
                }
                Err(e) => {
                    eprintln!("复制到剪贴板失败: {}", e);
                    // 返回错误，让 REPL 知道命令执行失败
                    return Err(e);
                }
            }
        }

        Command::Help => {
            println!("可用命令：");
            println!("  /add <path>      选中文件或文件夹");
            println!("  /remove <path>   移除已选文件或文件夹");
            println!("  /context         查看当前选中的文件和目录 (包括文件数和 Token 数)");
            println!("  /copy            将选中内容打包为 XML 并复制到剪贴板");
            println!("  /help            查看帮助");
            println!("  /quit            退出");
        }

        Command::Quit => {
            println!("收到 /quit 命令，程序即将退出...");
            // REPL层会检测此命令并退出
        }

        Command::Unknown(cmd_str) => {
            println!("未知命令: {}", cmd_str);
        }
    }

    Ok(())
}

/// 重新计算所有已选文件的计数与 token 数，将结果更新到 state
async fn recalc_file_and_tokens(state: Arc<Mutex<AppState>>) -> Result<(), AppError> {
    info!("开始重新计算 file_count 和 token_count...");
    // 1. 先把所有选中文件收集成 Vec<PathBuf>
    let all_files: Vec<PathBuf> = {
        let st = state.lock().unwrap();
        st.selected_paths.iter().cloned().collect()
    };
    info!("  -> 当前选中文件列表大小: {}", all_files.len());

    // 2. 调用 tokenizer 计算 token 总数 (async)
    let total_tokens = tokenizer::calculate_tokens(&all_files).await?;
    info!("  -> 计算得到的 total_tokens: {}", total_tokens);

    // 3. 最后再锁住 state，更新 file_count & token_count
    {
        let mut st = state.lock().unwrap();
        st.file_count = all_files.len(); // file_count 就是 all_files 的长度
        st.token_count = total_tokens;
        info!("  -> AppState 更新完毕: file_count={}, token_count={}", st.file_count, st.token_count);
    }
    Ok(())
} 