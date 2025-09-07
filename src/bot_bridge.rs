use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BotCommand {
    JoinVoiceChannel {
        guild_id: String,
        channel_id: String,
        requester: String, // User ID who requested
    },
    LeaveVoiceChannel {
        guild_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BotResponse {
    JoinSuccess {
        guild_id: String,
        channel_id: String,
    },
    JoinError {
        guild_id: String,
        error: String,
    },
    LeaveSuccess {
        guild_id: String,
    },
}

#[allow(dead_code)]
pub type BotCommandSender = mpsc::UnboundedSender<BotCommand>;
#[allow(dead_code)]
pub type BotCommandReceiver = mpsc::UnboundedReceiver<BotCommand>;
#[allow(dead_code)]
pub type BotResponseSender = mpsc::UnboundedSender<BotResponse>;
#[allow(dead_code)]
pub type BotResponseReceiver = mpsc::UnboundedReceiver<BotResponse>;

#[allow(dead_code)]
#[derive(Clone)]
pub struct SharedState {
    pub command_sender: BotCommandSender,
    pub pending_responses: Arc<RwLock<HashMap<String, tokio::sync::oneshot::Sender<BotResponse>>>>,
}

impl SharedState {
    #[allow(dead_code)]
    pub fn new() -> (Self, BotCommandReceiver) {
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        (
            Self {
                command_sender,
                pending_responses: Arc::new(RwLock::new(HashMap::new())),
            },
            command_receiver,
        )
    }

    #[allow(dead_code)]
    pub async fn send_command_and_wait(
        &self,
        command: BotCommand,
        timeout_ms: u64,
    ) -> Result<BotResponse, String> {
        let command_id = match &command {
            BotCommand::JoinVoiceChannel { guild_id, .. } => format!("join_{}", guild_id),
            BotCommand::LeaveVoiceChannel { guild_id } => format!("leave_{}", guild_id),
        };

        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        // Store the response channel
        {
            let mut pending = self.pending_responses.write().await;
            pending.insert(command_id.clone(), response_tx);
        }

        // Send command
        if self.command_sender.send(command).is_err() {
            // Clean up on send failure
            let mut pending = self.pending_responses.write().await;
            pending.remove(&command_id);
            return Err("Bot command channel closed".to_string());
        }

        // Wait for response with timeout
        match tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), response_rx).await
        {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err("Response channel closed".to_string()),
            Err(_) => {
                // Clean up on timeout
                let mut pending = self.pending_responses.write().await;
                pending.remove(&command_id);
                Err("Command timeout".to_string())
            }
        }
    }

    #[allow(dead_code)]
    pub async fn send_response(&self, response: BotResponse) {
        let response_id = match &response {
            BotResponse::JoinSuccess { guild_id, .. } | BotResponse::JoinError { guild_id, .. } => {
                format!("join_{}", guild_id)
            }
            BotResponse::LeaveSuccess { guild_id } => format!("leave_{}", guild_id),
        };

        let mut pending = self.pending_responses.write().await;
        if let Some(sender) = pending.remove(&response_id) {
            let _ = sender.send(response);
        }
    }
}
