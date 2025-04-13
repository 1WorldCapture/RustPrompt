use crate::error::AppError;
use arboard::Clipboard;
use anyhow::anyhow; // 显式导入 anyhow

pub fn copy_to_clipboard(xml: &str) -> Result<(), AppError> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| AppError::General(anyhow!("Failed to create clipboard object: {:?}", e)))?;
    
    clipboard.set_text(xml.to_owned())
        .map_err(|e| AppError::General(anyhow!("Failed to copy to clipboard: {:?}", e)))?;
    
    Ok(())
} 