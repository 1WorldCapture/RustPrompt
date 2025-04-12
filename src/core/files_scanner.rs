use std::path::{Path, PathBuf};
use anyhow::anyhow;

use crate::error::AppError;
use super::ignore_rules::IgnoreConfig;

/// 扫描给定路径，返回所有文件（不含文件夹），并应用忽略规则
/// 例如：隐藏文件、.gitignore、node_modules 等。
///
/// 如果 path 是单一文件，则检查是否忽略；
/// 如果 path 是文件夹，则递归扫描并排除忽略项。
pub async fn scan_dir(path: &Path, ignore_config: &IgnoreConfig) -> Result<Vec<PathBuf>, AppError> {
    let path = path.to_owned();
    let config = ignore_config.clone();

    let result = tokio::task::spawn_blocking(move || {
        if !path.exists() {
            return Err(AppError::General(anyhow!("路径不存在: {:?}", path)));
        }

        if path.is_file() {
            if config.should_ignore_path(&path) {
                Ok(vec![])
            } else {
                Ok(vec![path])
            }
        } else {
            // 如果是文件夹
            let walker = config.build_walker(&path).build();
            let mut files = Vec::new();
            for entry in walker {
                let entry = entry.map_err(|e|
                    AppError::General(anyhow!("walk entry error: {:?}", e))
                )?;
                if let Some(ft) = entry.file_type() {
                    if ft.is_file() {
                        files.push(entry.path().to_path_buf());
                    }
                }
            }
            Ok(files)
        }
    }).await.map_err(|e| {
        AppError::General(anyhow!("扫描任务失败: {:?}", e))
    })??;

    Ok(result)
} 