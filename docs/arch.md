
# 系统设计方案

## `cmdprompt` - 系统设计方案

**1. 核心理念与架构**

*   **REPL 中心化:** 核心交互模型是一个由 `reedline` 管理的读取-求值-打印循环 (Read-Eval-Print Loop)。
*   **状态管理:** 一个中心的、共享的、可变的状态 (`AppState`) 持有应用程序的数据（选中的文件、计数等）。访问需要同步（根据异步需求，可能使用 `tokio::sync::Mutex` 或 `std::sync::Mutex`）。
*   **异步操作:** IO 密集型（文件系统扫描、文件读取）和潜在的 CPU 密集型（Token 计算）任务被卸载到 `tokio` 工作线程，以保持主 REPL 循环的响应性。
*   **模块化:** 功能被分解为逻辑模块（REPL 交互、命令处理、核心逻辑、工具等），以提高清晰度、可测试性和可扩展性。
*   **命令模式:** 用户输入被解析为不同的 `Command` 枚举，然后分派给特定的处理程序/执行器。

**2. 建议的目录结构**

```
cmdprompt/
├── Cargo.toml
└── src/
    ├── main.rs          # 入口点，CLI 参数解析，初始化，主循环启动
    ├── error.rs         # 自定义错误类型 (使用 thiserror/anyhow)
    ├── config.rs        # (可选) 配置加载/管理
    │
    ├── app/
    │   ├── mod.rs
    │   └── state.rs     # 定义 AppState 结构体及其方法
    │
    ├── repl/
    │   ├── mod.rs
    │   ├── engine.rs    # 管理 Reedline 实例和主 REPL 循环
    │   ├── prompt.rs    # 实现 reedline::Prompt (动态提示符)
    │   ├── completion.rs # 实现 reedline::Completer (命令 & 路径补全)
    │   └── highlighter.rs # (可选) 实现 reedline::Highlighter
    │   └── history.rs     # (可选) 如果需要，自定义历史记录处理
    │
    ├── command/
    │   ├── mod.rs
    │   ├── parser.rs    # 将原始输入字符串解析为 Command 枚举
    │   └── executor.rs  # 执行解析后的 Command，与 AppState & core 交互
    │   └── definition.rs # 定义 Command 枚举及其参数
    │
    ├── core/
    │   ├── mod.rs
    │   ├── files.rs     # 文件系统交互 (扫描, 读取，使用 `ignore`, `tokio::fs`)
    │   ├── tokenizer.rs # Token 计算逻辑 (`tiktoken-rs`)
    │   ├── xml.rs       # XML 生成 (`quick-xml`)
    │   └── clipboard.rs # 剪贴板交互 (`arboard`)
    │
    └── common/          # (可选) 共享的工具、常量等
        └── mod.rs
```

**3. 关键组件及其职责**

*   **`main.rs`:**
    *   使用 `clap` 解析初始的 `<path>` 参数。
    *   初始化 `tokio` 运行时。
    *   创建共享的 `AppState` (很可能是 `Arc<tokio::sync::Mutex<AppState>>`)，可能用初始路径填充它。
    *   初始化 `ReplEngine`。
    *   调用 `ReplEngine::run()` 方法启动主循环。
    *   处理优雅关闭。

*   **`app::state::AppState`:**
    *   `selected_paths: HashSet<PathBuf>`: 存储绝对的、规范化的路径以避免重复。
    *   `file_count: usize`: 缓存的选中路径内的文件数量。
    *   `token_count: usize`: 缓存的总 Token 数量。
    *   提供添加/移除路径的方法，这些方法应触发对 `file_count` 和 `token_count` 的**异步**更新。关键在于，这些方法理想情况下应该只生成更新任务并快速返回，而不是阻塞 REPL。实际的更新可能稍后发生。

*   **`repl::engine::ReplEngine`:**
    *   持有 `reedline::Reedline` 实例。
    *   持有必要组件的引用/克隆（例如 `Arc<Mutex<AppState>>`, `command::Parser`, `command::Executor`）。
    *   使用自定义的 `Prompt`, `Completer`, `Highlighter`, `History` 配置 `Reedline`。
    *   包含主 `async fn run()` 循环：
        *   调用 `reedline.read_line(&prompt)`。
        *   处理 `Signal::Success(buffer)`:
            *   调用 `command::Parser::parse(buffer)`。
            *   如果 `Ok(command)`，则调用 `command::Executor::execute(command, app_state.clone())`。
            *   处理解析/执行错误（向用户显示它们）。
        *   处理 `Signal::CtrlC`, `Signal::CtrlD` 用于中断/退出。

*   **`repl::prompt::CmdPrompt` (实现 `reedline::Prompt`):**
    *   接收 `Arc<Mutex<AppState>>`。
    *   在其 `render_prompt_left` (或类似) 方法中：
        *   获取对 `AppState` 的**短暂**锁定。
        *   读取 `file_count` 和 `token_count`。
        *   格式化提示符字符串 (例如, `[5 个文件 | 1234 tokens] > `)。
        *   释放锁。

*   **`repl::completion::CmdPromptCompleter` (实现 `reedline::Completer`):**
    *   接收 `Arc<Mutex<AppState>>`。
    *   `complete(&self, line: &str, pos: usize)` 方法：
        *   解析 `line` 直到 `pos` 以确定上下文（命令开始 `/`，`/add ` 后的路径）。
        *   如果补全命令：过滤预定义的命令列表。
        *   如果补全路径：
            *   从 `line` 中确定基础目录和部分路径。
            *   执行**快速、非阻塞**的文件系统扫描（例如，使用 `std::fs::read_dir` 扫描当前目录，可能限制结果数量）。**避免在此处进行深度扫描**以防阻塞。
            *   根据部分路径过滤结果。
            *   返回 `Vec<reedline::Suggestion>`。

*   **`command::definition::Command` (枚举):**
    *   定义如 `Add(PathBuf)`, `Remove(PathBuf)`, `ShowContext`, `Copy`, `Quit`, `Help`, `Unknown(String)` 等变体。

*   **`command::parser`:**
    *   接收原始输入 `String`。
    *   分割成命令名和参数。
    *   映射到 `Command` 枚举。返回 `Result<Command, AppError>`。

*   **`command::executor`:**
    *   接收一个 `Command` 和 `Arc<Mutex<AppState>>`。
    *   使用 `match` 语句处理命令变体。
    *   **关键：为长时间运行的操作生成 `tokio` 任务：**
        *   `/add`, `/remove`: 生成一个使用 `ignore` 扫描文件的任务，然后短暂锁定 `AppState` 更新 `selected_paths`，接着生成*另一个*任务来计算数量/Token。
        *   `/copy`: 生成一个任务来读取文件 (`tokio::fs`)、生成 XML (`quick-xml`) 并复制到剪贴板 (`arboard`)。
        *   `/context`: 锁定 `AppState`，读取 `selected_paths`，打印，解锁。（很可能是同步的）。
        *   `/quit`: 通知 `ReplEngine` 循环终止。
    *   返回 `Result<(), AppError>`。如果需要异步进行复杂的状态更新，可能需要使用通道 (`tokio::sync::mpsc`) 进行通信。

*   **`core::*` 模块:**
    *   包含使用所选库（`ignore`, `tiktoken-rs`, `quick-xml`, `arboard`）的实际逻辑。
    *   这里的函数应设计为在适当时可能为 `async`（例如 `async fn calculate_tokens(paths: &HashSet<PathBuf>) -> Result<usize, AppError>`）。

*   **`error.rs`:**
    *   使用 `thiserror` 定义顶层的 `AppError` 枚举。
    *   变体可以包装来自底层库的特定错误（IO、解析、剪贴板等）。
    *   隐式或显式地使用 `anyhow::Error` 以便轻松地进行带上下文的错误传播。

**4. 异步策略**

*   主 `ReplEngine` 循环异步运行 (`async fn run`)。
*   `reedline.read_line()` 可能需要适应异步上下文，如果 `reedline` 本身不直接支持 `async`。这可能涉及必要时使用 `tokio::task::spawn_blocking` 将其放到阻塞线程上运行，或者检查 `reedline` 是否有与异步兼容的 API。（***更新:*** 请查阅 `reedline` 文档，它可能与异步事件源集成得很好）。
*   所有潜在的阻塞操作（`/add`, `/remove` 扫描, `/copy` 文件读取/XML生成/剪贴板, Token 计算）**必须**使用 `tokio::spawn` 生成到 `tokio` 的线程池上。
*   状态更新 (`AppState`) 需要使用 `tokio::sync::Mutex` 进行仔细同步。锁的持有时间应尽可能短。避免在持有锁的同时进行繁重计算。模式：
    1.  为计算生成异步任务（例如扫描）。
    2.  任务完成，获取结果。
    3.  获取 `AppState` 的锁。
    4.  快速更新状态字段。
    5.  释放锁。
    6.  (可选) 如果需要，触发提示符重绘（例如通知主循环）。

**5. 可扩展性考虑**

*   **新命令:** 向 `command::definition::Command` 添加变体，更新 `command::parser`，在 `command::executor` 中添加处理分支。如果需要，在 `core` 模块中实现核心逻辑。更新 `CmdPromptCompleter` 以建议新命令。
*   **配置:** 添加 `config.rs` 以加载设置（例如默认路径、XML 格式选项，如果以后添加 LLM 交互则需要 API 密钥），使用 `serde` 和类似 `config-rs` 或 `toml` 的库。在需要的地方传递 `Config` 结构体。
*   **不同的 Tokenizer:** 修改/替换 `core::tokenizer.rs`。
*   **不同的输出格式:** 修改 `core::xml.rs`。
*   **直接 LLM 交互:** 添加用于 API 客户端 (`reqwest`)、请求构建、响应处理的模块。这可能涉及对 `core` 的大量添加以及可能的新命令。
*   **UI 变更:** 修改 `repl::prompt` 或 `repl::highlighter`。添加更复杂的 UI 元素可能让你重新考虑在 Reedline 循环中（如果可行）使用 Ratatui (路线 A) 的组件，但初期保持简单。
