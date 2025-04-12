# 开发计划

## Sprint 1 - 基础脚手架与核心命令解析

**目标**  
1. 搭建最小可行的工程结构，完成 REPL 的基础循环。  
2. 实现基本命令的解析与处理框架，包括 `/add`、`/remove`、`/context`、`/copy`、`/`(提示) 等命令的“占位”逻辑。  
3. 提供基本的命令自动补全能力（命令名补全）。  
4. 能够在提示符（Prompt）上展示一些基础信息（如已选文件数，先不做 token 数量的动态计算，只留占位/伪值即可）。  

### 主要工作内容

1. **项目骨架（Cargo + 目录结构）**  
   - 初始化 `Cargo.toml` (项目名 `cmdprompt` 等基本信息)。  
   - 创建 `src/` 目录及子模块：`app/`、`repl/`、`command/`、`core/`、`common/`、`error.rs`、`main.rs`。  
   - 建立一个最小的 `main.rs`，只包含 `fn main()` / `#[tokio::main] async fn main()` 并打印“Hello cmdprompt”。  

2. **Reedline 集成**  
   - 在 `Cargo.toml` 中添加 `reedline` 依赖。  
   - 在 `repl::engine.rs` 中创建一个 `ReplEngine` 结构体，用来包装 `Reedline` 实例。  
   - 在 `ReplEngine::run()` 方法中实现一个最基础的循环：  
     ```rust
     loop {
         let sig = reedline.readline("cmd> "); // 暂时使用简单prompt
         match sig {
             Ok(line) => {
                 // 暂时直接打印 line 或用一个简单的handler
                 println!("You typed: {}", line);
             }
             Err(e) => {
                 println!("Error: {:?}", e);
                 break;
             }
         }
     }
     ```
   - 处理 `CtrlC` / `CtrlD` 的退出。  

3. **AppState 占位**  
   - 在 `app::state.rs` 里定义一个 `pub struct AppState { ... }`：  
     ```rust
     pub struct AppState {
         pub selected_paths: HashSet<PathBuf>,
         // 仅占位
         pub file_count: usize,
         pub token_count: usize,
     }
     ```
   - 暂时只做简单初始化：`selected_paths` 空集合，`file_count` 和 `token_count` 默认 0。  
   - 在 `main.rs` 里创建一个 `Arc<Mutex<AppState>>` 并传入 `ReplEngine`。  

4. **命令枚举与解析**  
   - 在 `command::definition.rs` 中定义 `Command` 枚举，包含以下变体：  
     ```rust
     pub enum Command {
         Add(PathBuf),
         Remove(PathBuf),
         ShowContext,
         Copy,
         Help,
         Quit,
         Unknown(String),
     }
     ```
   - 在 `command::parser.rs` 中实现一个简单的 `parse(input: &str) -> Result<Command, AppError>`：  
     1. 判断是否以 `/` 开头（若不是，则直接返回 `Command::Unknown`）。  
     2. 根据空格拆分命令名和后续参数。  
     3. 匹配 `/add`, `/remove`, `/context`, `/copy`, `/help`, `/quit` 等。  
     4. 将参数（若有）转换为 `PathBuf`。  

5. **命令执行器 (Executor) 占位**  
   - 在 `command::executor.rs` 中实现 `execute(command: Command, state: Arc<Mutex<AppState>>) -> Result<(), AppError>`：  
     - 暂时只对 `Add`, `Remove`, `ShowContext`, `Copy` 等命令做占位式输出，表示“此命令功能待实现”。  
     - 对 `Help` / `Unknown` / `Quit` 命令执行相应的处理/提示。  

6. **将解析和执行串起来**  
   - 在 `ReplEngine::run()` 中，每次读取一行后：  
     ```rust
     if line.trim().is_empty() {
         continue;
     }
     match parser::parse(&line) {
         Ok(cmd) => {
             if let Err(e) = executor::execute(cmd, app_state.clone()) {
                 println!("Execution error: {:?}", e);
             }
         }
         Err(e) => {
             println!("Parse error: {:?}", e);
         }
     }
     ```
   - 当命令是 `Quit` 时，让循环退出。  

7. **简单 Prompt & 命令自动补全**  
   - 在 `repl::prompt.rs` 创建一个实现 `reedline::Prompt` 的结构体 `CmdPrompt`，在 `render_prompt_left()` 方法中展示当前的 file_count（先写死 0）。  
   - 在 `repl::completion.rs` 创建一个实现 `reedline::Completer` 的结构体 `CmdPromptCompleter`，仅对命令名进行补全：`["/add", "/remove", "/context", "/copy", "/help", "/quit"]`。  
   - 在 `ReplEngine` 的初始化中注册该 Prompt 和 Completer：  
     ```rust
     let mut line_editor = Reedline::create();
     line_editor.set_prompt(Box::new(CmdPrompt { ... }));
     line_editor.set_completer(Box::new(CmdPromptCompleter { ... }));
     ```
   - 使得在输入 `/` 后按 <kbd>Tab</kbd> 时可以看到命令名建议。  

### 验收标准  

- 启动程序后，出现交互式命令行，输入任意行后能进行简单的“命令识别”。  
- 可以输入 `/add some/path`，在终端看到提示“Add 命令尚未实现” (占位)。  
- 输入 `/quit`，正常退出。  
- 输入 `/` + <kbd>Tab</kbd>，能显示命令补全建议。  
- Prompt 中已显示简单的 `[0 文件 | 0 tokens] >` 之类信息(还未真实计算)。  

---

## Sprint 2 - 文件选择与 Token 计算

**目标**  
1. 实现 `/add` & `/remove` 真正扫描文件系统并将结果加入/移除到 `AppState`。  
2. 完成 token 计算逻辑，实时更新 `token_count`。  
3. 在 Prompt 中准确展示 `file_count` 与 `token_count`。  
4. 优化路径自动补全，通过路径扫描给出建议。  

### 主要工作内容

1. **文件系统扫描 (core/files.rs)**  
   - 引入 `ignore` 或直接用 `std::fs` 做递归：  
     - `fn scan_dir(path: &Path) -> Result<Vec<PathBuf>, AppError>`  
       - 如果是文件，返回单个文件路径。  
       - 如果是文件夹，则递归收集所有文件路径（可结合 `ignore` 跳过 `.git`、`.DS_Store` 等）。  
     - 注意在 `/add` 或 `/remove` 操作中，如果是文件夹，则扫描所有子文件并批量添加/移除。  

2. **异步任务处理**  
   - 当执行 `/add <path>`：  
     - `tokio::spawn(async move { // 执行扫描 + state 更新 })` 或者在同一个 `async fn` 中执行扫描，并短暂锁住 `AppState`。  
     - 将扫描到的文件路径插入 `state.selected_paths` 中。  
     - 然后调度 Token 计算任务（也可以在同一个任务里做）：  
       ```rust
       let new_token_count = tokenizer::calculate_tokens(&newly_added_paths).await?;
       // 此时需要锁 state 并 += 到 token_count
       ```  
   - 同理 `/remove <path>` 做相反操作，或者只标记要移除的路径，然后更新 token 数。  
   - 每次执行完增删后，需要重新计算 `file_count`（即 `state.selected_paths.len()`）和新的 `token_count`。  

3. **Token 计算 (core/tokenizer.rs)**  
   - 引入 `tiktoken-rs` (或其它 Token 计算库)。  
   - `fn calculate_tokens(paths: &[PathBuf]) -> Result<usize, AppError>`：  
     - 读取文件内容（文本类文件，先忽略二进制）  
     - 使用 `tiktoken-rs` 等进行分词，累加数量。  
   - 如果文件过多，需要考虑异步批量计算；也可在本阶段简单实现整批同步读取 + 计算，再用 `tokio::spawn_blocking` 包裹。  

4. **实时更新 Prompt**  
   - Prompt 中通过定期(或命令执行完毕后)获取 `AppState` 锁来读取 `file_count` 和 `token_count` 并显示。  
   - 可能需要一个方式在状态发生更新后，提示当前 REPL 行进行重绘 (Reedline 提供 `trigger_refresh()` 或重新绘制 prompt 的方式)。  

5. **路径自动补全**  
   - 在 `repl::completion::CmdPromptCompleter` 中，如果当前输入是 `/add ` 或 `/remove ` 后跟路径，则启用文件系统路径补全：  
     - 从部分路径解析出一个基础目录 `base_dir` + 剩余过滤字符串 `prefix`。  
     - 对 `std::fs::read_dir(base_dir)` 做一次快速扫描，把所有文件/文件夹名称拿出来，根据 `prefix` 做 starts_with / contains 等过滤。  
     - 生成 `Reedline::Suggestion` 列表返回。  
   - **注意**：只做一层扫描，避免深层递归拖慢体验。  

6. **命令 `/context`**  
   - 在 `command::executor` 的 `/context` 分支中，读取 `state.selected_paths` 并打印出来。  
   - 同时打印当前的 `file_count` 与 `token_count`。  

### 验收标准  

- 可以通过 `/add /path/to/folder` 将文件夹下所有文件加入到 `AppState` 中；输入 `/context` 能查看选中文件列表，以及正确的文件数与 token 数。  
- 可以 `/remove` 对应的文件或目录后，重新查看 `/context`，文件数与 token 数正确减少。  
- Prompt 中展示的 `file_count` 与 `token_count` 与实际选中的内容一致。  
- 在输入 `/add some/path` 时，<kbd>Tab</kbd> 能够列出文件夹/文件作为补全。  

---

## Sprint 3 - XML 打包复制与整体完善

**目标**  
1. 实现 `/copy` 命令：读取已选文件内容，生成符合需求的 XML 格式，并复制到系统剪贴板。  
2. 完成命令提示的候选框功能（在输入 `/` 时列出所有命令，允许上下左右键选择，不用完整输入）。  
3. 加强错误处理与日志记录（对 IO / Token 计算 / XML 生成等出现异常时提示用户）。  
4. 优化用户体验（帮助信息、退出确认等）。  

### 主要工作内容

1. **XML 生成 (core/xml.rs)**  
   - 使用 `quick-xml` 或其它 XML 库来组装。<documents>文档根标签，<document>代表一个文件，它有一个属性index表示这个文件的序号。<source>代表文件路径，<document_content>代表文件内容。其中`project-tree-structure.txt`是个虚拟文件，它是整个项目文件的目录结构，采用树状格式打印输出。示例如下：  
     ```xml
     <documents>
      <document index="1">
      <source>project-tree-structure.txt</source>
      <document_content>
      hello-ratatui
          ├── src
          │   └── main.rs
          ├── Cargo.lock
          ├── Cargo.toml
          ├── LICENSE
          └── README.md
      </document_content>
      </document>
      <document index="2">
      <source>d:\workspace-playground\hello-ratatui\Cargo.toml</source>
      <document_content>
      [package]
      name = "hello-ratatui"
      version = "0.1.0"
      description = "An example generated using the simple template"
      authors = ["1WorldCapture <ll_nwpu@qq.com>"]
      license = "MIT"
      edition = "2021"

      [dependencies]
      crossterm = "0.28.1"
      ratatui = "0.29.0"
      color-eyre = "0.6.3"

      </document_content>
      </document>
      <document index="3">
      <source>d:\workspace-playground\hello-ratatui\src\main.rs</source>
      <document_content>
      use color_eyre::Result;
      use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
      use ratatui::{
          DefaultTerminal, Frame,
          style::Stylize,
          text::Line,
          widgets::{Block, Paragraph},
      };

      fn main() -> color_eyre::Result<()> {
          color_eyre::install()?;
          let terminal = ratatui::init();
          let result = App::new().run(terminal);
          ratatui::restore();
          result
      }

      /// The main application which holds the state and logic of the application.
      #[derive(Debug, Default)]
      pub struct App {
          /// Is the application running?
          running: bool,
      }

      impl App {
          /// Construct a new instance of [`App`].
          pub fn new() -> Self {
              Self::default()
          }

          /// Run the application's main loop.
          pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
              self.running = true;
              while self.running {
                  terminal.draw(|frame| self.render(frame))?;
                  self.handle_crossterm_events()?;
              }
              Ok(())
          }

          /// Renders the user interface.
          ///
          /// This is where you add new widgets. See the following resources for more information:
          ///
          /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
          /// - <https://github.com/ratatui/ratatui/tree/main/ratatui-widgets/examples>
          fn render(&mut self, frame: &mut Frame) {
              let title = Line::from("Ratatui Simple Template")
                  .bold()
                  .blue()
                  .centered();
              let text = "Hello, Ratatui!\n\n\
                  Created using https://github.com/ratatui/templates\n\
                  Press `Esc`, `Ctrl-C` or `q` to stop running.";
              frame.render_widget(
                  Paragraph::new(text)
                      .block(Block::bordered().title(title))
                      .centered(),
                  frame.area(),
              )
          }

          /// Reads the crossterm events and updates the state of [`App`].
          ///
          /// If your application needs to perform work in between handling events, you can use the
          /// [`event::poll`] function to check if there are any events available with a timeout.
          fn handle_crossterm_events(&mut self) -> Result<()> {
              match event::read()? {
                  // it's important to check KeyEventKind::Press to avoid handling key release events
                  Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
                  Event::Mouse(_) => {}
                  Event::Resize(_, _) => {}
                  _ => {}
              }
              Ok(())
          }

          /// Handles the key events and updates the state of [`App`].
          fn on_key_event(&mut self, key: KeyEvent) {
              match (key.modifiers, key.code) {
                  (_, KeyCode::Esc | KeyCode::Char('q'))
                  | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
                  // Add other key handlers here.
                  _ => {}
              }
          }

          /// Set running to false to quit the application.
          fn quit(&mut self) {
              self.running = false;
          }
      }

      </document_content>
      </document>
      </documents>
      <instruction>

      </instruction>
     ```
   - 如果文件过多，务必考虑内存消耗，可以先一次性构建字符串（小规模），或分批处理。  
   - 若需要对不同文件类型生成不同的节点结构，也可在此扩展。  

2. **剪贴板复制 (core/clipboard.rs)**  
   - 依赖 `arboard` 或 `clipboard` crate。  
   - 提供 `fn copy_to_clipboard(xml: &str) -> Result<(), AppError>`。  

3. **`/copy` 命令执行流程**  
   - 在 `command::executor` 中：  
     1. 获取 `state.selected_paths` 列表（要短暂锁一下）。  
     2. 异步读取这些文件内容并生成 XML 字符串。  
     3. 调用剪贴板 API 复制到系统剪贴板。  
     4. 向用户打印“已复制到剪贴板”或错误信息。  

4. **命令候选框（在输入 `/` 时列出全部命令）**  
   - 在 `CmdPromptCompleter` 中，如果检测到光标位于 `/` 后方且没有更多字符，则全部命令名都返回给补全列表，这样在用户按 <kbd>Tab</kbd> 或上下键时能选择。  
   - 同时要支持当用户继续输入字符时，动态过滤命令。  

5. **用户体验 & 健壮性**  
   - **帮助信息**：对 `/help` 命令做实际实现，打印可用命令列表及简要说明。  
   - **错误处理**：在扫描文件时、生成 XML 时、复制到剪贴板时若出现异常，统一捕获并打印简要错误信息。  
   - **退出确认**：可选地在 `/quit` 前提示“是否确认退出？(y/n)”。  

6. **测试与文档**  
   - 为核心逻辑编写一些单元测试，例如：  
     - `core::files::scan_dir` 对多种场景的测试（空目录、大目录等）。  
     - `core::tokenizer::calculate_tokens` 对不同大小文件、不同内容的测试。  
     - `core::xml::generate_xml` 对生成后的 XML 进行简单断言。  
   - 在 `README.md` 中更新使用示例，列出常见命令说明。  

### 验收标准

- 使用 `/add` 选定多个文件后，输入 `/copy` 能成功把组装好的 XML 放到系统剪贴板。  
- `Ctrl+V` 或系统“粘贴”操作能看到对应的 XML 结构。  
- 在输入 `/` 并按 <kbd>Tab</kbd> 或 <kbd>↓</kbd> / <kbd>↑</kbd> 时，可以循环在命令候选上选择。  
- `/help` 能显示所有命令说明。  
- 当文件路径无效或无法读写时，有明确的错误提示。  
- Sprint 1 + Sprint 2 + Sprint 3 所有功能综合流畅、无严重 BUG。  

---

## 总结

以上三个 **Sprints** 的划分，从**基础REPL脚手架**→**文件与token逻辑**→**XML打包与剪贴板复制**，循序渐进地完成了主要需求。通过该迭代过程，可以在每个阶段都获得一个可运行的“最小可用版本”，并在后续迭代中不断扩展与完善。