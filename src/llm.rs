use anyhow::Result;
use openai::embeddings::Embeddings;
use shuttle_secrets::SecretStore;

use crate::{error::{SetupError, EmbeddingError}, contents::File};

pub fn setup(secrets: &SecretStore) -> Result<()> {
    let open_ai_token = secrets.get("OPENAI_TOKEN").ok_or(SetupError("OPENAI_TOKEN not available"))?;
    
    openai::set_key(open_ai_token);
    
    Ok(())
}

pub async fn embed_file(file: &File) -> Result<Embeddings> {
    let sentences_as_str: Vec<&str> = file.sentences.iter().map(|s| s.as_str()).collect();

    Ok(
        Embeddings::create("text-embedding-ada-002", sentences_as_str, "Ding")
            .await
            .map_err(|_| EmbeddingError(""))?
    )
}