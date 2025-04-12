use std::sync::{Arc, Mutex};

use anyhow::Result;
use tokio::runtime::Runtime;

use crate::{
    app::state::AppState,
    repl::engine::ReplEngine,
};

mod app;
mod command;
mod error;
mod repl;

/// 程序入口点
fn main() -> Result<()> {
    // 使用一个 tokio 运行时来支持后续的异步操作
    let rt = Runtime::new()?;
    rt.block_on(async {
        // 初始化共享状态
        let app_state = Arc::new(Mutex::new(AppState::new()));

        // 创建并运行 REPL 引擎
        let mut engine = ReplEngine::new(app_state);
        engine.run().await?;

        Ok(())
    })
}