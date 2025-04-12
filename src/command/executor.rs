use std::sync::{Arc, Mutex};

use anyhow::Result;

use crate::app::state::AppState;
use crate::command::definition::Command;
use crate::error::AppError;

pub async fn execute(cmd: Command, state: Arc<Mutex<AppState>>) -> Result<(), AppError> {
    match cmd {
        Command::Add(path) => {
            println!("(占位) 执行 /add: {:?}", path);
            // 后续 Sprint 中实际实现: 扫描文件, 更新AppState.file_count等
        }
        Command::Remove(path) => {
            println!("(占位) 执行 /remove: {:?}", path);
            // 后续实现: 移除文件, 更新AppState
        }
        Command::ShowContext => {
            println!("(占位) 执行 /context. 已选文件如下：");
            let st = state.lock().unwrap();
            for p in &st.selected_paths {
                println!(" - {:?}", p);
            }
        }
        Command::Copy => {
            println!("(占位) 执行 /copy. 将把XML复制到剪贴板...");
        }
        Command::Help => {
            println!("可用命令：");
            println!("  /add <path>      选中文件或文件夹");
            println!("  /remove <path>   移除已选文件或文件夹");
            println!("  /context         查看当前选中的文件和目录");
            println!("  /copy            将选中内容打包为XML并复制到剪贴板");
            println!("  /help            查看帮助");
            println!("  /quit            退出");
        }
        Command::Quit => {
            println!("收到 /quit 命令，程序即将退出...");
            // 这里可以做一些收尾操作
            // 最简单的方式：什么都不做，靠 REPL engine 在检查后退出
            // 或者发一个通知signal告诉 ReplEngine 停止循环
        }
        Command::Unknown(cmd_str) => {
            println!("未知命令: {}", cmd_str);
            // 也可在这里 return Err(AppError::UnknownCommand(cmd_str)) 由REPL层打印错误
        }
    }

    Ok(())
} 