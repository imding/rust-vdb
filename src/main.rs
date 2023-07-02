use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use axum::{routing::post, Router, response::IntoResponse, extract::State, Json};

use axum_macros::debug_handler;
use axum_streams::StreamBodyAs;
use contents::File;
use futures::Stream;
use openai::chat::ChatCompletionDelta;
use serde::Deserialize;
use tokio::sync::mpsc::Receiver;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tower_http::services::ServeDir;
use vector::VectorDB;

use crate::{finder::Finder, error::PromptError, llm::{embed_sentence, chat_stream}};

mod contents;
mod error;
mod finder;
mod llm;
mod vector;

#[derive(Deserialize)]
struct Prompt {
    content: String
}

struct AppState {
    vector_db: VectorDB,
    files: Vec<File>
}

async fn embed_knowledge_base(vector_db: &mut VectorDB, files: &Vec<File>) -> Result<()> {
    for file in files {
        let embeddings = llm::embed_file(file).await?;

        println!("Embedding: {:?}", file.path);

        for embedding in embeddings.data {
            vector_db.upsert_embedding(embedding, file).await?;
        }
    }

    Ok(())
}

fn chat_completion_stream(chat_completion: Receiver<ChatCompletionDelta>) -> impl Stream<Item = String> {
    ReceiverStream::new(chat_completion)
        .map(|completion| completion.choices)
        .map(|choices| {
            choices
                .into_iter()
                .map(|choice| choice.delta.content.unwrap_or("\n".to_string()))
                .collect()
        })
}

fn error_stream() -> impl Stream<Item = String> {
    futures::stream::once(async move { "Error with your prompt".to_string() })
}

async fn get_completion(prompt: &str, app_state: &AppState) -> anyhow::Result<Receiver<ChatCompletionDelta>> {
    let embedding = embed_sentence(prompt).await?;
    let result = app_state.vector_db.search(embedding).await?;
    println!("Result: {:?}", result);
    let contents = app_state
        .files
        .get_contents(&result)
        .ok_or(PromptError {})?;
    
    chat_stream(prompt, contents.as_str()).await
}

#[debug_handler]
async fn prompt(State(app_state): State<Arc<AppState>>, Json(prompt): Json<Prompt>) -> impl IntoResponse {
    let prompt = prompt.content;
    let chat_completion = get_completion(&prompt, &app_state).await;

    if let Ok(chat_completion) = chat_completion {
        return StreamBodyAs::text(chat_completion_stream(chat_completion));
    }

    StreamBodyAs::text(error_stream())
}

#[shuttle_runtime::main]
async fn axum(
    #[shuttle_secrets::Secrets] secrets: shuttle_secrets::SecretStore,
    #[shuttle_static_folder::StaticFolder(folder = ".")] prefix: PathBuf,
    #[shuttle_static_folder::StaticFolder(folder = "kb")] kb_folder: PathBuf,
    #[shuttle_static_folder::StaticFolder(folder = "static")] assets: PathBuf
) -> shuttle_axum::ShuttleAxum {
    let files = contents::load_files_from_dir(kb_folder, &prefix, ".mdx")?;
    let mut vector_db = VectorDB::new(&secrets)?;

    llm::setup(&secrets)?;

    println!("Setup ✅");

    vector_db.reset_collection().await?;
    embed_knowledge_base(&mut vector_db, &files).await?;

    println!("Embeddings ✅");

    let app_state = AppState { vector_db, files };
    let app_state = Arc::new(app_state);
    let router = Router::new()
        .route("/prompt", post(prompt))
        .nest_service("/", ServeDir::new(assets))
        .with_state(app_state);

    Ok(router.into())
}