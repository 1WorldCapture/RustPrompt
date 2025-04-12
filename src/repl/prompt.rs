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
        let token_count = state.token_count;

        // 使用 format! 创建 String，然后转换为 Cow
        Cow::Owned(format!(
            "[{}] files | [{}] tokens] > ",
            file_count,
            token_count
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