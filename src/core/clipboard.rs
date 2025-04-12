use crate::error::AppError;
use arboard::Clipboard;
use anyhow::anyhow; // 显式导入 anyhow

pub fn copy_to_clipboard(xml: &str) -> Result<(), AppError> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| AppError::General(anyhow!("无法创建剪贴板对象: {:?}", e)))?;
    
    clipboard.set_text(xml.to_owned())
        .map_err(|e| AppError::General(anyhow!("复制到剪贴板失败: {:?}", e)))?;
    
    Ok(())
} 