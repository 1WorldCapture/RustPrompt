use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use anyhow::anyhow; // 显式导入 anyhow

use crate::error::AppError;

/// 扫描给定路径，返回所有文件（不含文件夹）的 PathBuf 列表。
/// - 如果 path 本身是一个文件，则返回 vec![path]；
/// - 如果 path 是文件夹，则递归扫描所有子文件夹 (遵守 .gitignore 等)，并返回其中所有文件；
/// - 如果 path 不存在或没有权限等，返回 Err(AppError)。
pub async fn scan_dir(path: &Path) -> Result<Vec<PathBuf>, AppError> {
    // 因为 ignore::WalkBuilder 是同步 API，这里使用 tokio::task::spawn_blocking
    // 把阻塞式扫描放到专门的线程池，以免阻塞主异步运行时。
    let path = path.to_owned();
    let result = tokio::task::spawn_blocking(move || {
        if !path.exists() {
            // 路径不存在
            return Err(AppError::General(anyhow!(
                "路径不存在: {:?}",
                path
            )));
        }

        if path.is_file() {
            // 如果是单一文件
            return Ok(vec![path]);
        }

        // 如果是文件夹，使用 ignore::WalkBuilder 递归扫描
        // 这里会自动识别 .gitignore
        let mut files = Vec::new();
        let walker = WalkBuilder::new(&path) // 传入引用
            .standard_filters(true) // 忽略 .git, *.bak, 等常见过滤
            .hidden(false)          // 视需求决定是否跳过隐藏文件
            .build();
        for entry in walker {
            let entry = entry.map_err(|e| {
                AppError::General(anyhow!("walk entry error: {:?}", e))
            })?;
            if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                files.push(entry.path().to_path_buf());
            }
        }
        Ok(files)
    })
    // 先 await 获取 Result<Result<Vec<PathBuf>, AppError>, JoinError>
    // 然后 map JoinError 到 anyhow::Error
    // 最后使用 ? 展开 Result<Vec<PathBuf>, AppError>
    .await
    .map_err(|join_err| AppError::General(anyhow!("扫描任务失败: {:?}", join_err)))??; 

    Ok(result)
} 