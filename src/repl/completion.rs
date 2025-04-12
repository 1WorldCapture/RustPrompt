use reedline::{Completer, Span, Suggestion};

/// 最小版的命令补全器
pub struct CmdPromptCompleter {}

impl Completer for CmdPromptCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        // 基础命令补全: 仅当行以 '/' 开头且光标不在开头时
        if !line.starts_with('/') || pos == 0 {
            return Vec::new();
        }

        let commands = vec![
            "/add", 
            "/remove", 
            "/context", 
            "/copy", 
            "/help", 
            "/quit"
        ];

        let current_input = &line[0..pos]; // 获取光标前的输入

        commands.into_iter()
            .filter(|cmd| cmd.starts_with(current_input)) // 查找以当前输入开头的命令
            .map(|cmd| {
                Suggestion {
                    value: cmd.to_string(),
                    description: None,
                    extra: None, // 暂无额外信息
                    style: None, // Added style field
                    // Span 覆盖需要被替换的文本部分 (从'/'之后到光标位置)
                    span: Span { start: 1, end: pos }, 
                    append_whitespace: true, // 补全后添加空格，方便输入参数
                }
            })
            .collect()
    }
} 