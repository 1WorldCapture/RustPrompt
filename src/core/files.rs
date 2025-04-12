use std::path::{Path, PathBuf};

use anyhow::anyhow; // 显式导入 anyhow

use crate::error::AppError;
use super::ignore_rules::IgnoreConfig;

/// 扫描给定路径，返回所有文件（不含文件夹），并应用忽略规则
/// 例如：隐藏文件、.gitignore、node_modules 等
///
/// - 如果 path 本身是一个文件，则返回 vec![path];
/// - 如果 path 是文件夹，则递归扫描所有子文件夹，忽略掉不需要的;
pub async fn scan_dir(path: &Path, ignore_config: &IgnoreConfig) -> Result<Vec<PathBuf>, AppError> {
    // 首先检查路径是否存在:
    let path = path.to_owned();
    // 将 ignore_config 克隆到阻塞线程
    let config = ignore_config.clone(); 
    let result = tokio::task::spawn_blocking(move || {
        if !path.exists() {
            return Err(AppError::General(anyhow!("路径不存在: {:?}", path)));
        }

        // 如果是单一文件, 直接返回
        if path.is_file() {
            // 使用 should_ignore_path 判断
            if config.should_ignore_path(&path) {
                return Ok(vec![]);
            } else {
                return Ok(vec![path]);
            }
        }

        // 如果是文件夹，则使用 ignore_config.build_walker
        let walker = config.build_walker(&path).build();
        let mut files = Vec::new();
        for entry in walker {
            let entry = entry.map_err(|e| AppError::General(anyhow!("walk entry error: {:?}", e)))?;
            if let Some(ft) = entry.file_type() {
                if ft.is_file() {
                    files.push(entry.path().to_path_buf());
                }
            }
        }
        Ok(files)
    })
    .await
    .map_err(|e| AppError::General(anyhow!("扫描任务失败: {:?}", e)))??;

    Ok(result)
} 