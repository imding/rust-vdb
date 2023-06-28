use std::path::PathBuf;

use anyhow::Result;
use axum::{routing::get, Router};

use contents::File;
use vector::VectorDB;

mod contents;
mod error;
mod llm;
mod vector;

async fn hello_world() -> &'static str {
    "Hello, world!"
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

#[shuttle_runtime::main]
async fn axum(
    #[shuttle_static_folder::StaticFolder(folder = "kb")] kb_folder: PathBuf,
    #[shuttle_static_folder::StaticFolder(folder = ".")] prefix: PathBuf,
    #[shuttle_secrets::Secrets] secrets: shuttle_secrets::SecretStore
) -> shuttle_axum::ShuttleAxum {
    let router = Router::new().route("/", get(hello_world));

    let files = contents::load_files_from_dir(kb_folder, &prefix, ".mdx")?;
    let mut vector_db = VectorDB::new(&secrets)?;

    llm::setup(&secrets)?;

    println!("Setup ✅");

    vector_db.reset_collection().await?;
    embed_knowledge_base(&mut vector_db, &files).await?;

    println!("Embeddings ✅");

    Ok(router.into())
}