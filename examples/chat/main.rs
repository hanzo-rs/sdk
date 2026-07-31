//! chat — one completion.
//!
//! Operation: POST /v1/chat/completions (ai_createChatCompletion), the
//! OpenAI-compatible surface.
//!
//! ```bash
//! HANZO_API_KEY=sk-... cargo run -p hanzo-examples --example chat
//! ```

use hanzo_client::apis::open_ai_compatible_api;
use hanzo_client::models::{ai_chat_message::Role, AiChatCompletionRequest, AiChatMessage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = hanzo_examples::config(None);
    let model = std::env::var("HANZO_MODEL").unwrap_or_else(|_| "zen-1".into());

    let mut message = AiChatMessage::new(Role::User);
    message.content = Some(Some("In one sentence: what is Hanzo?".into()));

    let req = AiChatCompletionRequest::new(model, vec![message]);
    let resp = open_ai_compatible_api::ai_create_chat_completion(&cfg, req).await?;

    for choice in resp.choices.unwrap_or_default() {
        let Some(message) = choice.message else { continue };
        // content is a string for plain replies and an array of parts for
        // multimodal ones, so the spec leaves it open.
        match message.content.flatten() {
            Some(serde_json::Value::String(text)) => println!("{text}"),
            Some(other) => println!("{other}"),
            None => {}
        }
    }
    if let Some(usage) = resp.usage {
        println!(
            "\ntokens: {} prompt + {} completion",
            usage.prompt_tokens.unwrap_or_default(),
            usage.completion_tokens.unwrap_or_default()
        );
    }
    Ok(())
}
