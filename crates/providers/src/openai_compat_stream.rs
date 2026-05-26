use futures_util::StreamExt;
use securellm_core::{
    Error, FinishReason, MessageRole, ProviderStream, Request, StreamChunk, StreamDelta,
};
use serde::Deserialize;

pub async fn send_stream(
    provider: &'static str,
    request: &Request,
    builder: reqwest::RequestBuilder,
) -> securellm_core::Result<ProviderStream> {
    let response = builder
        .send()
        .await
        .map_err(|e| Error::Network(format!("{provider} streaming request failed: {e}")))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Provider {
            provider: provider.to_string(),
            message: format!("streaming API error ({status}): {body}"),
        });
    }

    let request_id = request.id;
    let mut bytes = response.bytes_stream();
    let stream = async_stream::try_stream! {
        let mut buffer = String::new();

        while let Some(item) = bytes.next().await {
            let chunk = item
                .map_err(|e| Error::Network(format!("{provider} stream read failed: {e}")))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk).replace("\r\n", "\n"));

            while let Some(frame_end) = buffer.find("\n\n") {
                let frame = buffer[..frame_end].to_string();
                buffer.drain(..frame_end + 2);

                let Some(data) = event_data(&frame) else {
                    continue;
                };

                if data.trim() == "[DONE]" {
                    return;
                }

                let upstream: OpenAiStreamChunk = serde_json::from_str(&data)
                    .map_err(|e| Error::Serialization(format!("failed to parse {provider} stream chunk: {e}; data={data}")))?;

                for choice in upstream.choices {
                    yield StreamChunk {
                        request_id,
                        chunk_id: upstream.id.clone(),
                        delta: StreamDelta {
                            role: choice.delta.role.as_deref().map(role_from_str),
                            content: choice.delta.content,
                        },
                        finish_reason: choice.finish_reason.as_deref().map(finish_reason_from_str),
                    };
                }
            }
        }
    };

    Ok(Box::pin(stream))
}

fn event_data(frame: &str) -> Option<String> {
    let data = frame
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n");

    if data.is_empty() {
        None
    } else {
        Some(data)
    }
}

fn role_from_str(role: &str) -> MessageRole {
    match role {
        "system" => MessageRole::System,
        "user" => MessageRole::User,
        "assistant" => MessageRole::Assistant,
        "function" => MessageRole::Function,
        _ => MessageRole::Assistant,
    }
}

fn finish_reason_from_str(reason: &str) -> FinishReason {
    match reason {
        "stop" => FinishReason::Stop,
        "length" => FinishReason::Length,
        "content_filter" => FinishReason::ContentFilter,
        "function_call" => FinishReason::FunctionCall,
        "tool_calls" => FinishReason::ToolUse,
        _ => FinishReason::Unknown,
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChunk {
    id: String,
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiStreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamDelta {
    role: Option<String>,
    content: Option<String>,
}
