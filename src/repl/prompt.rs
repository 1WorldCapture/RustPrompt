use std::borrow::Cow;
use std::sync::{Arc, Mutex};

use reedline::{Prompt, PromptEditMode, PromptHistorySearch};
use crate::app::state::AppState;

pub struct CmdPrompt {
    pub app_state: Arc<Mutex<AppState>>,
}

impl Prompt for CmdPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        let state = self.app_state.lock().unwrap();
        let file_count = state.file_count;
        let raw_token_count = state.token_count;

        // 转换 token_count 到格式化字符串
        let token_str = if raw_token_count < 1000 {
            raw_token_count.to_string()
        } else {
            let val = raw_token_count as f64 / 1000.0;
            format!("{:.1}k", val) // 1.2k
        };

        // 使用 format! 创建 String，然后转换为 Cow
        Cow::Owned(format!(
            "[{}] files | [{}] tokens] > ",
            file_count,
            token_str // 使用格式化后的字符串
        ))
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _prompt_mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("> ") // 稍微改变一下指示符
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed(". ")
    }

    fn render_prompt_history_search_indicator(
        &self,
        _history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        Cow::Borrowed(" history search>> ")
    }
} 