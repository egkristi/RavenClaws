//! # Interactive UIs — TUI & GUI
//!
//! Optional, feature-gated interactive interfaces built on a shared [`ChatEngine`].
//!
//! - **TUI** (`tui` feature) — a terminal interface built with `ratatui` + `crossterm`.
//! - **GUI** (`gui` feature) — a native graphical window built with `egui`/`eframe`.
//!
//! Both are off by default so the core binary stays small (~7.7 MB). Enable them
//! with `--features tui` / `--features gui`, then launch with `--tui` / `--gui`.

use std::sync::Arc;

use crate::agent::ConversationMemory;
use crate::llm::{ChatMessage, LLMProviderTrait};

/// A shared, provider-agnostic conversation engine used by both the TUI and GUI.
///
/// It owns the conversation memory and sends one turn at a time to the LLM,
/// streaming the assistant's reply back through a callback so UIs can render
/// progress without blocking.
pub struct ChatEngine {
    llm: Arc<dyn LLMProviderTrait>,
    memory: ConversationMemory,
}

impl ChatEngine {
    /// Create a new engine for a single provider with the given system prompt.
    #[allow(dead_code)] // public library API + used by feature-gated TUI/GUI
    pub fn new(llm: Arc<dyn LLMProviderTrait>, system_prompt: &str) -> Self {
        Self {
            llm,
            memory: ConversationMemory::new(system_prompt, 0),
        }
    }

    /// Reset the conversation history.
    #[allow(dead_code)]
    pub fn reset(&mut self, system_prompt: &str) {
        self.memory = ConversationMemory::new(system_prompt, 0);
    }

    /// The full message history (system + turns).
    #[allow(dead_code)]
    pub fn history(&self) -> &[ChatMessage] {
        self.memory.history()
    }

    /// Send a user message and return the assistant's reply.
    ///
    /// `on_token` is invoked with each streaming chunk (when the provider supports
    /// streaming) so the UI can render incrementally. Returns the full reply.
    #[allow(dead_code)]
    pub async fn send<F>(&mut self, input: &str, mut on_token: F) -> crate::error::Result<String>
    where
        F: FnMut(&str),
    {
        self.memory.add_user_message(input);
        let messages = self.memory.history().to_vec();

        // Try streaming first; fall back to a single `chat` call.
        match self.llm.chat_stream(messages).await {
            Ok(mut stream) => {
                use futures::StreamExt;
                let mut full = String::new();
                while let Some(chunk) = stream.next().await {
                    if let Ok(c) = chunk {
                        full.push_str(&c.content);
                        on_token(&c.content);
                    }
                }
                if full.is_empty() {
                    // Provider returned no chunks — fall back to a direct call.
                    let response = self.llm.chat(self.memory.history().to_vec()).await?;
                    full = response
                        .choices
                        .first()
                        .map(|c| c.message.content.clone())
                        .unwrap_or_default();
                    on_token(&full);
                }
                self.memory.add_assistant_message(&full);
                Ok(full)
            }
            Err(_) => {
                let response = self.llm.chat(self.memory.history().to_vec()).await?;
                let content = response
                    .choices
                    .first()
                    .map(|c| c.message.content.clone())
                    .unwrap_or_default();
                on_token(&content);
                self.memory.add_assistant_message(&content);
                Ok(content)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal fake provider that echoes the last user message.
    struct EchoProvider;
    #[async_trait::async_trait]
    impl LLMProviderTrait for EchoProvider {
        async fn chat(
            &self,
            messages: Vec<ChatMessage>,
        ) -> Result<crate::llm::ChatResponse, crate::llm::LLMError> {
            let last = messages
                .last()
                .map(|m| m.content.clone())
                .unwrap_or_default();
            Ok(crate::llm::ChatResponse {
                id: "echo".to_string(),
                object: "chat.completion".to_string(),
                created: 0,
                model: "echo".to_string(),
                choices: vec![crate::llm::Choice {
                    index: 0,
                    message: ChatMessage::new("assistant", format!("echo: {last}")),
                    finish_reason: Some("stop".to_string()),
                    tool_calls: None,
                }],
                usage: None,
            })
        }
        fn provider_name(&self) -> &str {
            "echo"
        }
        fn model(&self) -> &str {
            "echo"
        }
    }

    #[tokio::test]
    async fn test_chat_engine_send_falls_back_to_chat() {
        let mut engine = ChatEngine::new(Arc::new(EchoProvider), "system");
        let mut tokens = Vec::new();
        let reply = engine
            .send("hello", |t| tokens.push(t.to_string()))
            .await
            .unwrap();
        assert_eq!(reply, "echo: hello");
        assert_eq!(tokens, vec!["echo: hello".to_string()]);
        // History should now contain system + user + assistant.
        assert_eq!(engine.history().len(), 3);
    }

    #[tokio::test]
    async fn test_chat_engine_reset() {
        let mut engine = ChatEngine::new(Arc::new(EchoProvider), "first");
        let _ = engine.send("hi", |_| {}).await.unwrap();
        engine.reset("second");
        assert_eq!(engine.history().len(), 1);
        assert_eq!(engine.history()[0].content, "second");
    }
}

#[cfg(feature = "tui")]
pub mod tui {
    //! Terminal UI built with `ratatui` + `crossterm`.
    use std::io;
    use std::sync::Arc;

    use crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        style::{Color, Style},
        text::{Line, Span, Text},
        widgets::{Block, Borders, Paragraph},
        Terminal,
    };
    use tokio::sync::mpsc;

    use super::ChatEngine;
    use crate::llm::LLMProviderTrait;

    /// A message in the transcript.
    #[derive(Clone)]
    struct Msg {
        role: String,
        content: String,
    }

    /// Terminal input events forwarded from a reader thread.
    enum Key {
        Char(char),
        Enter,
        Backspace,
        Escape,
        CtrlC,
    }

    fn err(msg: &str) -> crate::error::RavenClawsError {
        crate::error::RavenClawsError::CommandExecution(format!("TUI: {msg}"))
    }

    /// Run the terminal UI in a tokio runtime until the user quits.
    pub async fn run(
        llm: Arc<dyn LLMProviderTrait>,
        system_prompt: String,
    ) -> crate::error::Result<()> {
        let mut engine = ChatEngine::new(llm, &system_prompt);

        // Reconstruct the transcript from the engine history.
        let mut msgs: Vec<Msg> = engine
            .history()
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| Msg {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let mut input = String::new();
        let mut busy = false;

        // Terminal input reader thread (crossterm `event::read` is blocking).
        let (key_tx, mut key_rx) = mpsc::unbounded_channel::<Key>();
        std::thread::spawn(move || loop {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                let mapped = match key.code {
                    KeyCode::Esc => Key::Escape,
                    KeyCode::Char('c')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        Key::CtrlC
                    }
                    KeyCode::Enter => Key::Enter,
                    KeyCode::Backspace => Key::Backspace,
                    KeyCode::Char(c) => Key::Char(c),
                    _ => continue,
                };
                if key_tx.send(mapped).is_err() {
                    break;
                }
            }
        });

        // Raw terminal mode.
        enable_raw_mode().map_err(|e| err(&format!("enable raw mode: {e}")))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
            .map_err(|e| err(&format!("enter alt screen: {e}")))?;
        let backend = ratatui::backend::CrosstermBackend::new(stdout);
        let mut terminal =
            Terminal::new(backend).map_err(|e| err(&format!("terminal init: {e}")))?;

        let result = run_loop(
            &mut terminal,
            &mut engine,
            &mut msgs,
            &mut input,
            &mut busy,
            &mut key_rx,
        )
        .await;

        // Restore terminal.
        disable_raw_mode().ok();
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .ok();
        terminal.show_cursor().ok();

        result
    }

    async fn run_loop(
        terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
        engine: &mut ChatEngine,
        msgs: &mut Vec<Msg>,
        input: &mut String,
        busy: &mut bool,
        key_rx: &mut mpsc::UnboundedReceiver<Key>,
    ) -> crate::error::Result<()> {
        // Channel for streaming tokens from the in-flight `send` future.
        let (tok_tx, mut tok_rx) = mpsc::unbounded_channel::<String>();
        let mut send_fut: Option<tokio::task::JoinHandle<crate::error::Result<String>>> = None;

        loop {
            terminal
                .draw(|f| draw(f, msgs, input, *busy))
                .map_err(|e| err(&format!("draw: {e}")))?;

            // Drain streaming tokens into the current assistant bubble.
            while let Ok(tok) = tok_rx.try_recv() {
                if let Some(last) = msgs.last_mut() {
                    if last.role == "assistant" {
                        last.content.push_str(&tok);
                    }
                }
            }

            // Poll terminal input (short timeout to stay responsive to tokens).
            let key = tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_millis(30)) => {
                    key_rx.try_recv().ok()
                }
                k = key_rx.recv() => k,
            };

            if let Some(k) = key {
                match k {
                    Key::Escape | Key::CtrlC => {
                        if let Some(h) = send_fut.take() {
                            h.abort();
                        }
                        break;
                    }
                    Key::Enter if !*busy && !input.trim().is_empty() => {
                        let text = input.clone();
                        input.clear();
                        msgs.push(Msg {
                            role: "user".to_string(),
                            content: text.clone(),
                        });
                        msgs.push(Msg {
                            role: "assistant".to_string(),
                            content: String::new(),
                        });
                        *busy = true;
                        let tx = tok_tx.clone();
                        // Clone the provider + replay history into a fresh engine so we
                        // don't hold a `&mut` borrow of `engine` across the await.
                        let llm = engine.llm.clone();
                        let system = engine.history()[0].content.clone();
                        let history: Vec<Msg> = msgs.clone();
                        send_fut = Some(tokio::spawn(async move {
                            let mut e = ChatEngine::new(llm, &system);
                            for m in &history {
                                if m.role == "user" {
                                    e.memory.add_user_message(&m.content);
                                } else if m.role == "assistant" && !m.content.is_empty() {
                                    e.memory.add_assistant_message(&m.content);
                                }
                            }
                            e.send(&text, |tok| {
                                let _ = tx.send(tok.to_string());
                            })
                            .await
                        }));
                    }
                    Key::Char(c) => input.push(c),
                    Key::Backspace => {
                        input.pop();
                    }
                    // Enter while busy or with empty input is a no-op.
                    Key::Enter => {}
                }
            }

            // Finalize a completed send.
            if let Some(h) = send_fut.as_mut() {
                if h.is_finished() {
                    let handle = send_fut.take().unwrap();
                    match handle.await {
                        Ok(Ok(full)) => {
                            if let Some(last) = msgs.last_mut() {
                                if last.role == "assistant" {
                                    last.content = full;
                                }
                            }
                            *busy = false;
                        }
                        Ok(Err(e)) => {
                            msgs.push(Msg {
                                role: "error".to_string(),
                                content: e.to_string(),
                            });
                            *busy = false;
                        }
                        Err(e) => {
                            msgs.push(Msg {
                                role: "error".to_string(),
                                content: format!("join error: {e}"),
                            });
                            *busy = false;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn draw(f: &mut ratatui::Frame, msgs: &[Msg], input: &str, busy: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(3), Constraint::Length(3)].as_ref())
            .split(f.area());

        let transcript: Text = msgs
            .iter()
            .map(|m| {
                let (tag, color) = match m.role.as_str() {
                    "user" => ("you", Color::Cyan),
                    "assistant" => ("agent", Color::Green),
                    _ => ("error", Color::Red),
                };
                Line::from(vec![
                    Span::styled(format!("[{tag}] "), Style::default().fg(Color::DarkGray)),
                    Span::styled(m.content.as_str(), Style::default().fg(color)),
                ])
            })
            .collect::<Vec<Line>>()
            .into();

        let para = Paragraph::new(transcript)
            .block(Block::default().borders(Borders::ALL).title("RavenClaws"));
        f.render_widget(para, chunks[0]);

        let status = if busy {
            "…"
        } else {
            "Esc to quit · Enter to send"
        };
        let input_widget = Paragraph::new(input).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Input {status}")),
        );
        f.render_widget(input_widget, chunks[1]);
    }
}

#[cfg(feature = "gui")]
pub mod gui {
    //! Graphical UI built with `eframe` / `egui` (immediate-mode, memory-safe Rust).
    //!
    //! The `ChatEngine` lives in a dedicated worker thread (it owns the conversation
    //! memory), and the UI communicates with it over MPSC channels: the UI sends
    //! `Cmd` requests, and the worker streams back `Event`s (tokens / completion).
    use std::sync::mpsc;
    use std::sync::Arc;

    use crate::llm::LLMProviderTrait;

    /// Commands sent from the UI to the worker thread.
    enum Cmd {
        /// Send a user message.
        Send(String),
        /// Reset the conversation.
        Reset,
        /// Shut the worker down.
        Quit,
    }

    /// Events streamed from the worker back to the UI.
    enum Event {
        /// A streamed token chunk for the current assistant reply.
        Token(String),
        /// A reply completed (Ok) or failed (Err).
        Done(Result<String, String>),
        /// The conversation was reset.
        Reset,
    }

    /// A message in the transcript.
    #[derive(Clone)]
    struct Msg {
        role: String,
        content: String,
    }

    /// Run the native graphical window.
    pub fn run(llm: Arc<dyn LLMProviderTrait>, system_prompt: String) -> crate::error::Result<()> {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([720.0, 560.0]),
            ..Default::default()
        };

        eframe::run_native(
            "RavenClaws",
            options,
            Box::new(move |cc| Ok(Box::new(GuiApp::new(cc, llm, system_prompt)))),
        )
        .map_err(|e| crate::error::RavenClawsError::CommandExecution(format!("GUI: {e}")))
    }

    struct GuiApp {
        input: String,
        busy: bool,
        msgs: Vec<Msg>,
        /// Sender to the worker thread.
        cmd_tx: mpsc::Sender<Cmd>,
        /// Receiver for events from the worker thread.
        event_rx: mpsc::Receiver<Event>,
        /// Worker join handle (aborted on exit).
        worker: Option<std::thread::JoinHandle<()>>,
    }

    impl GuiApp {
        fn new(
            _cc: &eframe::CreationContext<'_>,
            llm: Arc<dyn LLMProviderTrait>,
            system_prompt: String,
        ) -> Self {
            let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
            let (event_tx, event_rx) = mpsc::channel::<Event>();

            let worker = std::thread::spawn(move || {
                worker_loop(llm, system_prompt, cmd_rx, event_tx);
            });

            Self {
                input: String::new(),
                busy: false,
                msgs: Vec::new(),
                cmd_tx,
                event_rx,
                worker: Some(worker),
            }
        }

        fn send(&mut self) {
            if self.busy {
                return;
            }
            let text = self.input.trim().to_string();
            if text.is_empty() {
                return;
            }
            self.input.clear();
            self.msgs.push(Msg {
                role: "user".to_string(),
                content: text.clone(),
            });
            self.msgs.push(Msg {
                role: "assistant".to_string(),
                content: String::new(),
            });
            self.busy = true;
            let _ = self.cmd_tx.send(Cmd::Send(text));
        }

        fn drain(&mut self) {
            while let Ok(ev) = self.event_rx.try_recv() {
                match ev {
                    Event::Token(tok) => {
                        if let Some(last) = self.msgs.last_mut() {
                            if last.role == "assistant" {
                                last.content.push_str(&tok);
                            }
                        }
                    }
                    Event::Done(Ok(full)) => {
                        if let Some(last) = self.msgs.last_mut() {
                            if last.role == "assistant" {
                                last.content = full;
                            }
                        }
                        self.busy = false;
                    }
                    Event::Done(Err(e)) => {
                        self.msgs.push(Msg {
                            role: "error".to_string(),
                            content: e,
                        });
                        self.busy = false;
                    }
                    Event::Reset => {
                        self.msgs.clear();
                        self.busy = false;
                    }
                }
            }
        }
    }

    impl eframe::App for GuiApp {
        fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            self.drain();
            if self.busy {
                ctx.request_repaint();
            }
        }

        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            // Header.
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("🐦‍⬛ RavenClaws");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Reset").clicked() {
                        let _ = self.cmd_tx.send(Cmd::Reset);
                        self.msgs.clear();
                        self.busy = false;
                    }
                });
            });
            ui.add_space(6.0);

            // Transcript.
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for m in &self.msgs {
                        let (tag, color) = match m.role.as_str() {
                            "user" => ("you", egui::Color32::LIGHT_BLUE),
                            "assistant" => ("agent", egui::Color32::LIGHT_GREEN),
                            _ => ("error", egui::Color32::LIGHT_RED),
                        };
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(color, format!("[{tag}] "));
                            ui.label(&m.content);
                        });
                        ui.add_space(2.0);
                    }
                });

            // Input row.
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.input)
                        .hint_text("Type a message…")
                        .desired_width(f32::INFINITY),
                );
                let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let send_clicked = ui.button("Send").clicked();
                if (enter || send_clicked) && !self.busy {
                    self.send();
                    response.request_focus();
                }
            });
            ui.add_space(6.0);
        }

        fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
            let _ = self.cmd_tx.send(Cmd::Quit);
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }

    /// The worker thread owns the `ChatEngine` and processes commands.
    fn worker_loop(
        llm: Arc<dyn LLMProviderTrait>,
        system_prompt: String,
        cmd_rx: mpsc::Receiver<Cmd>,
        event_tx: mpsc::Sender<Event>,
    ) {
        let mut engine = crate::ui::ChatEngine::new(llm, &system_prompt);

        // Run a tokio runtime on this thread to drive the async `send`.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime for GUI worker");

        for cmd in cmd_rx {
            match cmd {
                Cmd::Quit => break,
                Cmd::Reset => {
                    engine.reset(&system_prompt);
                    let _ = event_tx.send(Event::Reset);
                }
                Cmd::Send(text) => {
                    let tx = event_tx.clone();
                    let result = rt.block_on(engine.send(&text, move |tok| {
                        let _ = tx.send(Event::Token(tok.to_string()));
                    }));
                    let _ = event_tx.send(Event::Done(result.map_err(|e| e.to_string())));
                }
            }
        }
    }
}
