use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct PromptSetting {
    // unlocked context size option 필요
    context_size: i64,

    max_response_length: i64,
    streaming: bool,
    temperature: f32,
    frequency_penalty: f32,
    presence_penalty: f32,
    top_p: f32,

    // "auto" | "minimum" | "low" | "medium" | "high" | "maximum"
    reasoning_effort: String,
    thinking: PromptThinking,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct PromptThinking {
    #[serde(rename = "type")]
    thinking_type: String,
}
