use reedline::{Completer, Span, Suggestion};
use std::fs;
use std::path::{Path, PathBuf};
use log::{debug, info}; // 导入日志宏
use std::sync::{Arc, Mutex}; // <-- Import Mutex
use crate::app::state::AppState; // <-- Import AppState
use crate::core::ignore_rules::IgnoreConfig; // 引入 IgnoreConfig

/// 补全器，支持命令和路径
pub struct CmdPromptCompleter {
    pub app_state: Arc<Mutex<AppState>>, // <-- Add app_state field
}

impl Completer for CmdPromptCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        info!("complete 方法被调用: line='{}', pos={}", line, pos);

        // 如果行首不是 '/', 就不是命令行，不补全
        if !line.starts_with('/') {
            debug!("非 / 开头，不进行补全。");
            return Vec::new();
        }

        // 从光标前的文本获取 当前输入
        let current_input_before_cursor = &line[..pos];
        debug!("光标前输入: '{}'", current_input_before_cursor);

        // 使用空格分割输入，但只分割一次，以分离命令和可能的参数部分
        let parts: Vec<&str> = current_input_before_cursor.splitn(2, ' ').collect();
        let cmd_part = parts.get(0).unwrap_or(&""); // 命令部分，例如 "/add" 或 "/remove /some/pa"
        let arg_part = parts.get(1).unwrap_or(&""); // 参数部分，例如 "src/" 或 ""

        debug!("解析结果: cmd_part='{}', arg_part='{}'", cmd_part, arg_part);

        // 判断是否需要进行路径补全
        if (*cmd_part == "/add" || *cmd_part == "/remove") && current_input_before_cursor.contains(' ') {
            // 包含空格，说明命令已输入完整，现在补全参数部分 (arg_part)
            // 注意：这里的 arg_part 可能包含空格，但 suggest_paths 会处理
            debug!("检测到路径补全场景...");
            let span_start = cmd_part.len() + 1;
            if *cmd_part == "/remove" {
                // 如果是 /remove，调用基于上下文的补全
                debug!("调用 suggest_context_paths...");
                return self.suggest_context_paths(arg_part, span_start, pos);
            } else {
                // 如果是 /add，调用基于文件系统的补全
                debug!("调用 suggest_paths (for /add)...");
                return self.suggest_paths(arg_part, span_start, pos);
            }
        } else if !current_input_before_cursor.contains(' ') {
             // 不包含空格，说明还在输入命令本身，补全命令
            debug!("检测到命令补全场景，调用 suggest_commands...");
             return self.suggest_commands(current_input_before_cursor, pos);
        } else {
             // 其他情况（例如命令后有空格但不是 /add 或 /remove），暂时不补全
             debug!("其他未处理的补全场景，返回空。");
             return Vec::new();
        }
    }
}

impl CmdPromptCompleter {
    /// 补全命令名
    fn suggest_commands(&self, input: &str, pos: usize) -> Vec<Suggestion> {
        let commands = vec!["/add", "/remove", "/context", "/copy", "/help", "/quit"];
        debug!("suggest_commands: input='{}'", input);
        let suggestions: Vec<Suggestion> = commands
            .iter()
            .filter(|cmd| cmd.starts_with(input))
            .map(|cmd| {
                debug!("  -> 建议: {}", cmd);
                Suggestion {
                    value: cmd.to_string(),
                    description: None,
                    extra: None,
                    style: None,
                    // 替换从 input 的开头到 pos
                    span: Span { start: 0, end: pos }, 
                    append_whitespace: true, // 补全命令后加空格
                }
            })
            .collect();
        debug!("suggest_commands: 返回 {} 条建议", suggestions.len());
        suggestions
    }

    /// 补全文件路径(只做一层)，并应用忽略规则
    fn suggest_paths(&self, partial_path: &str, span_start: usize, pos: usize) -> Vec<Suggestion> {
        debug!("suggest_paths: partial_path='{}', span_start={}, pos={}", partial_path, span_start, pos);
        let ignore_config = IgnoreConfig::default(); // 获取默认忽略配置

        // 获取当前工作目录作为默认基准
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // 将 partial_path 解析为基准目录和文件前缀
        let (base_dir, prefix) = {
            let path_to_parse = Path::new(partial_path);
            if path_to_parse.is_absolute() {
                let parent = path_to_parse.parent().unwrap_or(path_to_parse);
                let prefix = path_to_parse.file_name().unwrap_or_default().to_string_lossy();
                (parent.to_path_buf(), prefix.to_string())
            } else if partial_path.ends_with(std::path::MAIN_SEPARATOR) {
                // 如果以分隔符结尾，说明要列出目录内容，基准是这个目录，前缀为空
                (current_dir.join(path_to_parse), "".to_string())
            } else if partial_path.contains(std::path::MAIN_SEPARATOR) {
                 let parent = path_to_parse.parent().unwrap_or(Path::new("."));
                 let prefix = path_to_parse.file_name().unwrap_or_default().to_string_lossy();
                 (current_dir.join(parent), prefix.to_string())
            } else {
                (current_dir, partial_path.to_string())
            }
        };

        debug!("  -> 解析后: base_dir='{:?}', prefix='{}'", base_dir, prefix);

        let read_dir_result = fs::read_dir(&base_dir);
        let mut suggestions = Vec::new();

        if let Ok(entries) = read_dir_result {
            for entry_result in entries {
                if let Ok(entry) = entry_result {
                    let entry_path = entry.path();
                    // 应用忽略规则
                    if ignore_config.should_ignore_path(&entry_path) {
                        continue;
                    }

                    if let Ok(file_type) = entry.file_type() {
                        let file_name = entry.file_name().to_string_lossy().to_string();
                        
                        // 如果 prefix 为空，或者文件名以 prefix 开头
                        if prefix.is_empty() || file_name.starts_with(&prefix) {
                            let mut display_name = file_name;
                            // 如果是目录，在末尾加上分隔符
                            if file_type.is_dir() {
                                display_name.push(std::path::MAIN_SEPARATOR);
                            }
                            
                            // 构造替换后的完整参数值 (包含用户输入的目录部分)
                            let value_to_insert = {
                                let path_prefix_typed_by_user = if let Some(idx) = partial_path.rfind(std::path::MAIN_SEPARATOR) {
                                    &partial_path[..=idx]
                                } else {
                                    ""
                                };
                                format!("{}{}", path_prefix_typed_by_user, display_name)
                            };
                            
                            debug!("    -> 匹配到: {}, 插入值: {}", display_name, value_to_insert);

                            suggestions.push(Suggestion {
                                value: value_to_insert, // 使用构造好的完整相对路径
                                description: None,
                                extra: None,
                                style: None,
                                // 替换从参数部分的开始到当前光标
                                span: Span { start: span_start, end: pos }, 
                                append_whitespace: !file_type.is_dir(), // 文件后加空格，目录后不加
                            });
                        }
                    }
                }
            }
        }
        debug!("suggest_paths: 返回 {} 条建议", suggestions.len());
        suggestions
    }

    /// 根据当前选中的路径 (AppState.selected_paths) 进行补全
    fn suggest_context_paths(&self, partial_path: &str, span_start: usize, pos: usize) -> Vec<Suggestion> {
        debug!("suggest_context_paths: partial_path='{}', span_start={}, pos={}", partial_path, span_start, pos);
        
        let selected_paths = {
            let state = self.app_state.lock().unwrap();
            // 克隆 HashSet 以快速释放锁
            state.selected_paths.clone() 
        };
        
        debug!("  -> 当前选中路径数量: {}", selected_paths.len());

        let mut suggestions = Vec::new();
        
        for path in selected_paths {
            // 将 PathBuf 转换为字符串以进行比较
            let path_str = path.to_string_lossy();

            // 检查路径字符串是否以用户输入的 partial_path 开头
            if path_str.starts_with(partial_path) {
                 debug!("    -> 匹配到: {}", path_str);
                 suggestions.push(Suggestion {
                    value: path_str.to_string(), // 补全的值是完整的已选路径
                    description: None,
                    extra: None,
                    style: None,
                    // 替换从参数部分的开始到当前光标
                    span: Span { start: span_start, end: pos }, 
                    append_whitespace: false, // remove 通常不需要加空格
                });
            }
        }
        debug!("suggest_context_paths: 返回 {} 条建议", suggestions.len());
        suggestions
    }
} 