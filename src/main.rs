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
    // 初始化日志记录器
    env_logger::init();
    log::info!("日志系统已初始化");

    // 使用一个 tokio 运行时来支持后续的异步操作
    let rt = Runtime::new()?;
    rt.block_on(async {
        log::info!("进入 Tokio 运行时");
        // 初始化共享状态
        let app_state = Arc::new(Mutex::new(AppState::new()));
        log::info!("共享状态已创建");

        // 创建并运行 REPL 引擎
        let mut engine = ReplEngine::new(app_state);
        log::info!("REPL 引擎已创建，即将运行...");
        engine.run().await?;
        log::info!("REPL 引擎运行结束");

        Ok(())
    })
}