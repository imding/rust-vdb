use anyhow::{Ok, Result};
use openai::{
    chat::{
        ChatCompletionBuilder, ChatCompletionDelta, ChatCompletionMessage,
        ChatCompletionMessageRole,
    },
    embeddings::{Embedding, Embeddings},
};
use shuttle_secrets::SecretStore;
use tokio::sync::mpsc::Receiver;

use crate::{
    contents::File,
    error::{EmbeddingError, SetupError},
};

type Conversation = Receiver<ChatCompletionDelta>;

pub fn setup(secrets: &SecretStore) -> Result<()> {
    let open_ai_token = secrets
        .get("OPENAI_TOKEN")
        .ok_or(SetupError("OPENAI_TOKEN not available"))?;

    openai::set_key(open_ai_token);

    Ok(())
}

pub async fn embed_file(file: &File) -> Result<Embeddings> {
    let sentences_as_str: Vec<&str> = file.sentences.iter().map(|s| s.as_str()).collect();
    let embeddings = Embeddings::create("text-embedding-ada-002", sentences_as_str, "Ding")
        .await
        .map_err(|_| EmbeddingError(""))?;

    Ok(embeddings)
}

pub async fn embed_sentence(sentence: &str) -> Result<Embedding> {
    let embedding = Embedding::create("text-embedding-ada-002", sentence, "Ding")
        .await
        .map_err(|_| EmbeddingError(""))?;

    Ok(embedding)
}

pub async fn chat_stream(prompt: &str, contents: &str) -> Result<Conversation> {
    let content = Some(format!(
        "Given the following context:\n{}\n\nRespond to the following:\n{}",
        contents, prompt
    ));

    Ok(ChatCompletionBuilder::default()
        .model("gpt-3.5-turbo")
        .temperature(0.0)
        .user("stefan")
        .messages(vec![ChatCompletionMessage {
            role: ChatCompletionMessageRole::User,
            content,
            name: None,
            function_call: None,
        }])
        .create_stream()
        .await
        .map_err(|_| EmbeddingError(""))?)
}
