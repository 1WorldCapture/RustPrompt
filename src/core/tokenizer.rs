use crate::error::AppError;

// Removed unused calculate_tokens function
/*
pub async fn calculate_tokens(paths: &[PathBuf]) -> Result<usize, AppError> {
    // ... implementation ...
}
*/

// NEW: 直接对字符串计算 Token 数
pub fn calculate_tokens_in_string(s: &str) -> Result<usize, AppError> {
    let bpe = tiktoken_rs::get_bpe_from_model("gpt-3.5-turbo")
        .map_err(|e| AppError::General(anyhow::anyhow!("无法加载BPE: {:?}", e)))?;

    let tokens = bpe.encode_ordinary(s);
    Ok(tokens.len())
} 