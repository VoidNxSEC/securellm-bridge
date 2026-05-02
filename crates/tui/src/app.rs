//! TUI Application state

use anyhow::Result;
use securellm_agents::{tools::EchoTool, AgentExecutor, ToolRegistry};
use securellm_context_manager::ContextManager;
use securellm_providers::llamacpp::LlamaCppProvider;
use securellm_task_manager::TaskManager;
use securellm_voice_agents::VoiceAgent;
use std::sync::Arc;

use crate::components::*;
use crate::multiplex::TabBar;
use crate::InputMode;

pub struct TuiApp {
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub chat_panel: ChatPanel,
    pub task_panel: TaskPanel,
    pub context_panel: ContextPanel,
    pub logs_panel: LogsPanel,
    pub tool_panel: ToolExecutionPanel,
    pub overview_panel: OverviewPanel,
    pub status_bar: StatusBar,
    pub focused_panel: FocusedPanel,

    // Multiplex system
    pub tab_bar: TabBar,

    // Agent system
    pub agent_mode: bool,
    agent_executor: Option<AgentExecutor>,

    // Backend services
    task_manager: TaskManager,
    context_manager: ContextManager,
    voice_agent: Option<VoiceAgent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPanel {
    Chat,
    Tasks,
    Tools,
    Context,
    Logs,
}

impl TuiApp {
    pub fn new() -> Result<Self> {
        let task_manager = TaskManager::new();
        let context_manager = ContextManager::new()?;
        let voice_agent = VoiceAgent::new().ok(); // Optional

        // Initialize agent system
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool::new());

        let provider = LlamaCppProvider::new(8081, "llamacppturbo").ok();
        let agent_executor = provider.map(|p| AgentExecutor::new(registry, Arc::new(p)));

        Ok(Self {
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            chat_panel: ChatPanel::new(),
            task_panel: TaskPanel::new(),
            context_panel: ContextPanel::new(),
            logs_panel: LogsPanel::new(),
            tool_panel: ToolExecutionPanel::new(),
            overview_panel: OverviewPanel::new(),
            status_bar: StatusBar::new(),
            focused_panel: FocusedPanel::Chat,
            tab_bar: TabBar::new(),
            agent_mode: false,
            agent_executor,
            task_manager,
            context_manager,
            voice_agent,
        })
    }

    pub async fn update(&mut self) -> Result<()> {
        // Update panel states
        self.task_panel.update(&self.task_manager);
        self.context_panel.update(&self.context_manager);
        Ok(())
    }

    pub async fn send_message(&mut self) -> Result<()> {
        if !self.input_buffer.is_empty() {
            let message = self.input_buffer.clone();
            self.chat_panel.add_message("user", &message);
            self.logs_panel.add_log(
                "INFO",
                &format!("Queued user message ({} chars)", message.len()),
            );
            self.input_buffer.clear();

            // Send to LlamaCpp provider
            use securellm_core::{
                LLMProvider, Message as LLMMessage, MessageContent, MessageRole, Request,
            };
            use securellm_providers::llamacpp::LlamaCppProvider;

            match LlamaCppProvider::new(8081, "llamacppturbo") {
                Ok(provider) => {
                    let request = Request::new("llamacpp", "llamacppturbo")
                        .add_message(LLMMessage {
                            role: MessageRole::System,
                            content: MessageContent::Text(
                                "You are a helpful assistant. Always respond in English or Portuguese. Never respond in Chinese or other languages.".to_string()
                            ),
                            name: None,
                            metadata: None,
                        })
                        .add_message(LLMMessage {
                            role: MessageRole::User,
                            content: MessageContent::Text(message.clone()),
                            name: None,
                            metadata: None,
                        });

                    match provider.send_request(request).await {
                        Ok(response) => {
                            if let Ok(text) = response.text() {
                                self.chat_panel.add_message("assistant", &text);
                                self.logs_panel
                                    .add_log("INFO", "Provider response received");
                            }
                        }
                        Err(e) => {
                            self.logs_panel
                                .add_log("ERROR", &format!("Provider request failed: {}", e));
                            self.chat_panel
                                .add_message("system", &format!("Error: {}", e));
                        }
                    }
                }
                Err(e) => {
                    self.logs_panel
                        .add_log("ERROR", &format!("Provider initialization failed: {}", e));
                    self.chat_panel
                        .add_message("system", &format!("Provider error: {}", e));
                }
            }
        }
        Ok(())
    }

    pub async fn toggle_voice(&mut self) -> Result<()> {
        if self.input_mode == InputMode::Voice {
            // Stop recording
            self.input_mode = InputMode::Normal;
            self.logs_panel.add_log("INFO", "Voice capture stopped");
            // TODO: Process audio
        } else {
            // Start recording
            self.input_mode = InputMode::Voice;
            self.logs_panel.add_log("INFO", "Voice capture started");
            // TODO: Start audio capture
        }
        Ok(())
    }

    pub fn cycle_focus(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Chat => FocusedPanel::Tasks,
            FocusedPanel::Tasks => FocusedPanel::Tools,
            FocusedPanel::Tools => FocusedPanel::Context,
            FocusedPanel::Context => FocusedPanel::Logs,
            FocusedPanel::Logs => FocusedPanel::Chat,
        };
        self.logs_panel
            .add_log("DEBUG", &format!("Focus -> {}", self.focused_panel_label()));
    }

    /// Toggle agent mode
    pub fn toggle_agent_mode(&mut self) {
        self.agent_mode = !self.agent_mode;
        if self.agent_mode {
            self.chat_panel
                .add_message("system", "🤖 Agent mode enabled");
            self.logs_panel.add_log("INFO", "Agent mode enabled");
        } else {
            self.chat_panel.add_message("system", "Agent mode disabled");
            self.logs_panel.add_log("INFO", "Agent mode disabled");
        }
    }

    pub fn provider_available(&self) -> bool {
        self.agent_executor.is_some()
    }

    pub fn voice_available(&self) -> bool {
        self.voice_agent.is_some()
    }

    pub fn message_count(&self) -> usize {
        self.chat_panel.message_count()
    }

    pub fn focused_panel_label(&self) -> &'static str {
        match self.focused_panel {
            FocusedPanel::Chat => "chat",
            FocusedPanel::Tasks => "tasks",
            FocusedPanel::Tools => "tools",
            FocusedPanel::Context => "context",
            FocusedPanel::Logs => "logs",
        }
    }
}
