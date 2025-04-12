use reedline::{Completer, Span, Suggestion};
use log::{debug, info}; // 导入日志宏

/// 最小版的命令补全器
pub struct CmdPromptCompleter {}

impl Completer for CmdPromptCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        info!("complete 方法被调用: line='{}', pos={}", line, pos);

        // 恢复原始逻辑: 仅当行以 '/' 开头且光标不在开头时
        if !line.starts_with('/') || pos == 0 {
            debug!("输入不满足补全条件 (非 / 开头或光标在行首)，返回空建议。");
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
        debug!("当前输入用于过滤: '{}'", current_input);

        let suggestions: Vec<Suggestion> = commands.into_iter()
            .filter(|cmd| cmd.starts_with(current_input)) // 查找以当前输入开头的命令
            .map(|cmd| {
                debug!("为 '{}' 生成 Suggestion", cmd);
                Suggestion {
                    value: cmd.to_string(),
                    description: None, // 可以稍后添加描述
                    extra: None, 
                    style: None, 
                    span: Span { start: 0, end: pos }, // 替换从头到光标的文本
                    append_whitespace: true, // 补全后添加空格
                }
            })
            .collect();
        
        debug!("过滤后生成的建议数量: {}", suggestions.len());
        suggestions
    }
} 