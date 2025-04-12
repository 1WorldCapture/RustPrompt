use std::sync::{Arc, Mutex};

use reedline::{Reedline, Signal};
use anyhow::Result;

use crate::{
    app::state::AppState,
    command::{parser, executor},
    repl::{
        prompt::CmdPrompt,
        completion::CmdPromptCompleter,
    },
};

pub struct ReplEngine {
    /// reedline 编辑器实例
    editor: Reedline,
    /// 全局共享状态
    app_state: Arc<Mutex<AppState>>,
    /// 动态提示符
    prompt: CmdPrompt,
    /// 是否正在运行，用于控制循环退出
    running: bool,
}

impl ReplEngine {
    pub fn new(app_state: Arc<Mutex<AppState>>) -> Self {
        // 直接创建编辑器，并设置补全器
        let editor = Reedline::create()
            .with_completer(Box::new(CmdPromptCompleter {}));

        // 创建 Prompt 对象
        let prompt = CmdPrompt {
            app_state: app_state.clone(),
        };

        Self {
            editor,
            app_state,
            prompt,
            running: true,
        }
    }

    /// 运行主循环
    pub async fn run(&mut self) -> Result<()> {
        while self.running {
            // 读取用户输入，传入 Prompt
            let sig = self.editor.read_line(&self.prompt);

            match sig {
                Ok(Signal::Success(buffer)) => {
                    // 若用户输入为空，仅跳过
                    if buffer.trim().is_empty() {
                        continue;
                    }
                    // 尝试解析命令
                    match parser::parse(&buffer) {
                        Ok(cmd) => {
                            // 执行命令
                            if let Err(e) = executor::execute(cmd.clone(), self.app_state.clone()).await {
                                eprintln!("执行命令时出错: {}", e);
                            }
                            // 特殊处理 Quit 命令以停止循环
                            if matches!(cmd, crate::command::definition::Command::Quit) {
                                self.running = false;
                            }
                        }
                        Err(e) => {
                            eprintln!("命令解析错误: {}", e);
                        }
                    }
                }
                Ok(Signal::CtrlC) | Ok(Signal::CtrlD) => {
                    // 用户按下 Ctrl+C / Ctrl+D，退出
                    println!("Bye!");
                    self.running = false;
                }
                Err(e) => {
                    eprintln!("读取输入时出错: {:?}", e);
                    self.running = false;
                }
            }
        }

        Ok(())
    }

    /// 提供给外部的方式，让其他逻辑可触发退出
    #[allow(dead_code)]
    pub fn stop(&mut self) {
        self.running = false;
    }
} 