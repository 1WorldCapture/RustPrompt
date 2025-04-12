use std::path::PathBuf;

use anyhow::Result;

use crate::error::AppError;
use crate::command::definition::Command;

pub fn parse(input: &str) -> Result<Command, AppError> {
    // 必须以'/'开头，否则视为 Unknown
    if !input.starts_with('/') {
        return Ok(Command::Unknown(input.to_string()));
    }

    // 按空格拆分: 第一个是命令, 剩下的是参数
    let mut parts = input.trim().split_whitespace();
    let cmd_str = parts.next().unwrap_or("");
    let arg_str = parts.next(); // 可能是文件路径

    match cmd_str {
        "/add" => {
            // 如果没有参数，就先返回一个空路径
            let p = arg_str.unwrap_or("").to_string();
            Ok(Command::Add(PathBuf::from(p)))
        }
        "/remove" => {
            let p = arg_str.unwrap_or("").to_string();
            Ok(Command::Remove(PathBuf::from(p)))
        }
        "/context" => Ok(Command::ShowContext),
        "/copy" => Ok(Command::Copy),
        "/reset" => Ok(Command::Reset),
        "/help" => Ok(Command::Help),
        "/quit" => Ok(Command::Quit),

        // 其它未知命令
        _ => {
            // 依旧用 Unknown 表示
            Ok(Command::Unknown(input.to_string()))
        }
    }
} 