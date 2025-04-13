use std::sync::{Arc, Mutex};

use reedline::{
    ColumnarMenu, Emacs, KeyCode, KeyModifiers, Reedline, ReedlineEvent, ReedlineMenu, Signal, 
    default_emacs_keybindings, // 用于获取默认绑定
    MenuBuilder, // <--- 导入 MenuBuilder trait
    Validator, ValidationResult, // <--- 导入 Validator
    EditCommand,
};
use anyhow::Result;
use log::debug; // <-- 导入 debug 宏

use crate::{
    app::state::{AppState, ReplMode, ReplEditorMode}, // <-- 导入 ReplEditorMode
    command::{parser, executor, definition::Command},
    repl::{
        prompt::CmdPrompt,
        completion::CmdPromptCompleter,
    },
    error::AppError,
};

// /// 用于区分单行/多行  <-- 已移至 state.rs
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum ReplEditorMode {
//     SingleLine,
//     MultiLine,
// }

/// 自定义Validator：最后一行若是 :submit => 视为完成
pub struct SubmitValidator;

impl Validator for SubmitValidator {
    fn validate(&self, content: &str) -> ValidationResult {
        if content.lines().last().map_or(false, |l| l.trim() == ":submit") {
            ValidationResult::Complete
        } else {
            ValidationResult::Incomplete
        }
    }
}

pub struct ReplEngine {
    /// reedline 编辑器实例
    editor: Reedline,
    /// 全局共享状态
    app_state: Arc<Mutex<AppState>>,
    /// 动态提示符
    prompt: CmdPrompt,
    /// 是否正在运行，用于控制循环退出
    running: bool,
    // [MODIFIED] 使用 state.rs 中的 editor_mode
    // editor_mode: ReplEditorMode, // <- 移到 AppState
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

        // 5. 创建 Reedline 实例，并配置所有组件 (初始为单行模式)
        let editor = Reedline::create()
            .with_completer(completer) // Use the new completer instance
            .with_menu(ReedlineMenu::EngineCompleter(completion_menu)) // 注册菜单
            .with_edit_mode(edit_mode); // 注册编辑模式 (包含自定义的 Tab 绑定)
            // .with_validator(Box::new(DefaultValidator::new())) // 默认不需要显式设置 Validator

        // 创建 Prompt 对象
        let prompt = CmdPrompt {
            app_state: app_state.clone(),
        };

        Self {
            editor,
            app_state,
            prompt,
            running: true,
            // editor_mode: ReplEditorMode::SingleLine, // <- 状态移至 AppState
        }
    }

    /// 进入多行编辑模式
    pub fn enter_multiline_mode(&mut self) -> Result<(), AppError> {
        // 更新 AppState.editor_mode
        {
            let mut st = self.app_state.lock().unwrap();
            st.editor_mode = ReplEditorMode::MultiLine;
        }

        // 准备 keybinding
        let mut kb = default_emacs_keybindings();
        // 禁用Tab
        kb.add_binding(
            KeyModifiers::NONE,
            KeyCode::Tab,
            ReedlineEvent::None,
        );
        let edit_mode = Box::new(Emacs::new(kb));

        // 重新创建 editor
        self.editor = Reedline::create()
            .with_edit_mode(edit_mode)
            .with_validator(Box::new(SubmitValidator))
            .with_completer(Box::new(CmdPromptCompleter {
                app_state: self.app_state.clone(),
            }));

        // 读取已有 prompt_text
        let existing_prompt = {
            let st = self.app_state.lock().unwrap();
            st.prompt_text.clone()
        };
        if !existing_prompt.is_empty() {
            // 预填充编辑器缓冲区
            self.editor.run_edit_commands(&[EditCommand::InsertString(existing_prompt)]);
        }

        Ok(())
    }

    /// [NEW] 退出多行模式 (恢复单行配置)
    fn exit_multiline_mode(&mut self) {
         debug!("Exiting multiline mode...");
         { // 更新 AppState 中的模式
            let mut st = self.app_state.lock().unwrap();
            st.editor_mode = ReplEditorMode::SingleLine;
         }

        let mut kb = default_emacs_keybindings();
        kb.add_binding(
            KeyModifiers::NONE,
            KeyCode::Tab,
            ReedlineEvent::UntilFound(vec![
                ReedlineEvent::Menu("completion_menu".to_string()),
                ReedlineEvent::MenuNext,
            ]),
        );
        let edit_mode = Box::new(Emacs::new(kb)); // 默认单行

        // 恢复单行的 Completer 和 Menu
        let completer = Box::new(CmdPromptCompleter {
            app_state: self.app_state.clone(),
        });
        let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));

        // 重新配置 editor, 移除 validator (或使用默认), 恢复 completer/menu
        self.editor = Reedline::create()
            .with_edit_mode(edit_mode)
            .with_completer(completer)
            .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
            // .with_validator(Box::new(DefaultValidator::new())) // 不需要显式移除或设置默认 Validator
            // 没有 .with_multiline(), 依赖 validator
            ;
    }


    /// 运行主循环
    pub async fn run(&mut self) -> Result<()> {
        while self.running {
            // 读取用户输入，传入 Prompt
            let sig = self.editor.read_line(&self.prompt);

            match sig {
                Ok(Signal::Success(buffer)) => {
                    // 检查是否在多行模式
                    let is_multiline = {
                        let st = self.app_state.lock().unwrap();
                        st.editor_mode == ReplEditorMode::MultiLine
                    };

                    if is_multiline {
                        // 处理多行编辑
                        let mut lines: Vec<&str> = buffer.lines().collect();
                        if let Some(last) = lines.last() {
                            if last.trim() == ":submit" {
                                lines.pop();
                                let final_text = lines.join("\n");
                                {
                                    let mut st = self.app_state.lock().unwrap();
                                    st.prompt_text = final_text;
                                    println!("(提示) 多行编辑提交完毕，新的 prompt:\n{}", st.prompt_text);
                                }
                                self.exit_multiline_mode();
                                continue;
                            }
                        }
                    }

                    // --- 处理单行模式下的输入 ---
                    debug!("Singleline mode received success signal. Buffer: '{}'", buffer);

                    // 若用户输入为空，仅跳过
                    if buffer.trim().is_empty() {
                        debug!("Empty input, skipping.");
                        continue;
                    }

                    // 如果当前模式是 Prompt 并且没有以'/'开头，就当做 AppendPromptText
                    let mut is_prompt_input = false;
                    let current_repl_mode = { // 获取当前的 REPL 模式 (Manual/Prompt)
                        let st = self.app_state.lock().unwrap();
                        st.mode.clone()
                    };

                    if current_repl_mode == ReplMode::Prompt && !buffer.starts_with('/') {
                        debug!("Detected prompt text input in Prompt mode.");
                        is_prompt_input = true;
                    }

                    if is_prompt_input {
                        let cmd = Command::AppendPromptText(buffer);
                        if let Err(e) = executor::execute(cmd, self.app_state.clone(), self).await {
                            eprintln!("执行命令时出错: {}", e);
                        }
                        continue; // 跳过常规 parse()
                    }

                    // 否则，正常解析命令
                    match parser::parse(&buffer) {
                        Ok(cmd) => {
                             debug!("Parsed command: {:?}", cmd);
                             
                             // --- 特殊处理 /prompt 命令以进入多行模式 ---
                             if matches!(&cmd, Command::Prompt) && current_repl_mode == ReplMode::Prompt {
                                 debug!("Detected /prompt command in Prompt mode. Entering multiline edit.");
                                 // 不通过 executor 执行，直接在这里切换模式
                                 self.enter_multiline_mode()?;
                                 continue; // 进入下一轮循环，等待多行输入
                             }

                            // --- 对于其他命令，正常执行 ---
                            if let Err(e) = executor::execute(cmd.clone(), self.app_state.clone(), self).await {
                                eprintln!("执行命令时出错: {}", e);
                            }
                            // 特殊处理 Quit 命令以停止循环
                            if matches!(cmd, Command::Quit) {
                                debug!("Quit command received. Stopping REPL.");
                                self.running = false;
                            }
                        }
                        Err(e) => {
                            eprintln!("命令解析错误: {}", e);
                        }
                    }
                }
                Ok(Signal::CtrlC) | Ok(Signal::CtrlD) => {
                    // 用户按下 Ctrl+C / Ctrl+D
                    let editor_mode = {
                        let st = self.app_state.lock().unwrap();
                        st.editor_mode
                    };
                    if editor_mode == ReplEditorMode::MultiLine {
                         // 在多行模式下按 Ctrl+C/D，应该取消编辑并返回单行模式
                         println!("(提示) 已取消多行编辑。");
                         self.exit_multiline_mode();
                         // 不退出程序，继续循环
                    } else {
                        // 在单行模式下按 Ctrl+C/D，退出程序
                        println!("Bye!");
                        self.running = false;
                    }
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