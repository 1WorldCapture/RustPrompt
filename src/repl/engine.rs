use std::sync::{Arc, Mutex};

use reedline::{
    ColumnarMenu, Emacs, KeyCode, KeyModifiers, Reedline, ReedlineEvent, ReedlineMenu, Signal, 
    default_emacs_keybindings, // 用于获取默认绑定
    MenuBuilder // <--- 导入 MenuBuilder trait
};
use anyhow::Result;

use crate::{
    app::state::AppState,
    command::{parser, executor, definition::Command},
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
        // 1. 创建 Completer, 传入 app_state
        let completer = Box::new(CmdPromptCompleter {
             app_state: app_state.clone(), // <-- Pass AppState here
        });

        // 2. 创建菜单 (用于显示补全)，并命名
        let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));

        // 3. 配置键位绑定 (让 Tab 触发菜单)
        let mut keybindings = default_emacs_keybindings();
        keybindings.add_binding(
            KeyModifiers::NONE, // 无需修饰键 (如 Shift, Ctrl)
            KeyCode::Tab,       // Tab 键
            ReedlineEvent::UntilFound(vec![ // 尝试一系列事件直到成功
                ReedlineEvent::Menu("completion_menu".to_string()), // 保持菜单名称引用，内部会处理
                ReedlineEvent::MenuNext, // 如果菜单已打开，则选择下一项
            ]),
        );

        // 4. 创建 Emacs 编辑模式，并传入修改后的键位绑定
        let edit_mode = Box::new(Emacs::new(keybindings));

        // 5. 创建 Reedline 实例，并配置所有组件
        let editor = Reedline::create()
            .with_completer(completer) // Use the new completer instance
            .with_menu(ReedlineMenu::EngineCompleter(completion_menu)) // 注册菜单
            .with_edit_mode(edit_mode); // 注册编辑模式 (包含自定义的 Tab 绑定)

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

                    // 如果当前模式是 Prompt 并且没有以'/'开头，就当做 AppendPromptText
                    let mut is_prompt_input = false;
                    {
                        let st = self.app_state.lock().unwrap();
                        if st.mode == crate::app::state::ReplMode::Prompt && !buffer.starts_with('/') {
                            is_prompt_input = true;
                        }
                        // Lock is dropped here
                    }

                    if is_prompt_input {
                        let cmd = Command::AppendPromptText(buffer);
                        if let Err(e) = executor::execute(cmd, self.app_state.clone()).await {
                            eprintln!("执行命令时出错: {}", e);
                        }
                        continue; // 跳过常规 parse()
                    }

                    // 否则，正常解析和执行命令
                    match parser::parse(&buffer) {
                        Ok(cmd) => {
                            // 执行命令
                            if let Err(e) = executor::execute(cmd.clone(), self.app_state.clone()).await {
                                eprintln!("执行命令时出错: {}", e);
                            }
                            // 特殊处理 Quit 命令以停止循环
                            if matches!(cmd, Command::Quit) {
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