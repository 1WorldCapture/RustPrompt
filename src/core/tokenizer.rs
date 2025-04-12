use std::path::PathBuf;
use tokio::fs;
use crate::error::AppError;
use log::warn; // 用于记录读取失败

/// 计算一组文件的 token 总数。
/// 简化实现：将所有文件视为 UTF-8 文本读取后再计算 Token；
/// 如果遇到二进制文件或读取错误时，可根据需求决定跳过或报错。
pub async fn calculate_tokens(paths: &[PathBuf]) -> Result<usize, AppError> {
    // 获取 BPE 实例
    let bpe = tiktoken_rs::get_bpe_from_model("gpt-3.5-turbo")
        .map_err(|e| AppError::General(anyhow::anyhow!("无法加载BPE: {:?}", e)))?;

    let mut total_tokens = 0usize;

    for path in paths {
        match fs::read_to_string(path).await {
            Ok(content) => {
                // 使用 bpe.encode_ordinary 计算 token
                let tokens = bpe.encode_ordinary(&content);
                total_tokens += tokens.len();
            }
            Err(err) => {
                warn!("读取 {:?} 失败 (可能不是文本文件?): {:?}", path, err);
            }
        }
    }

    Ok(total_tokens)
}

// NEW: 直接对字符串计算 Token 数
pub fn calculate_tokens_in_string(s: &str) -> Result<usize, AppError> {
    let bpe = tiktoken_rs::get_bpe_from_model("gpt-3.5-turbo")
        .map_err(|e| AppError::General(anyhow::anyhow!("无法加载BPE: {:?}", e)))?;

    let tokens = bpe.encode_ordinary(s);
    Ok(tokens.len())
} 