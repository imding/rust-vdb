use openai::embeddings::Embedding;
use qdrant_client::{
    prelude::{QdrantClient, QdrantClientConfig, Payload},
    qdrant::{CreateCollection, VectorsConfig, VectorParams, Distance, vectors_config::Config, PointStruct}
};
use serde_json::json;
use shuttle_secrets::SecretStore;
use anyhow::{Result, Ok};

use crate::{error::{SetupError, EmbeddingError}, contents::File};

static COLLECTION: &str = "docs";

pub struct VectorDB {
    id: u64,
    client: QdrantClient
}

impl VectorDB {
    pub fn new(secrets: &SecretStore) -> Result<Self> {
        let qdrant_token = secrets.get("QDRANT_TOKEN").ok_or(SetupError("QDRANT_TOKEN not available."))?;
        let qdrant_url = secrets.get("QDRANT_URL").ok_or(SetupError("ADRANT_URL not available."))?;
        let mut qdrant_config = QdrantClientConfig::from_url(&qdrant_url);

        qdrant_config.set_api_key(&qdrant_token);
        
        Ok(Self {
            id: 0,
            client: QdrantClient::new(Some(qdrant_config))?
        })
    }

    // in production, should prefer update vs reset entire DB
    pub async fn reset_collection(&self) -> Result<()> {
        self.client.delete_collection(COLLECTION).await?;

        self.client.create_collection(&CreateCollection {
            collection_name: COLLECTION.to_string(),
            vectors_config: Some(VectorsConfig {
                config: Some(Config::Params(VectorParams {
                    size: 1536,
                    // consult Qdrant documentation
                    distance: Distance::Cosine.into(),
                    hnsw_config: None,
                    quantization_config: None,
                    on_disk: None
                }))
            }),
            ..Default::default()
        }).await?;

        Ok(())
    }

    pub async fn upsert_embedding(&mut self, embedding: Embedding, file: &File) -> Result<()> {
        let payload: Payload = json!({ "id": file.path.clone() }).try_into().map_err(|_| EmbeddingError(""))?;
        let vec: Vec<f32> = embedding.vec.iter().map(|&x| x as f32).collect();
        let points = vec![PointStruct::new(self.id, vec, payload)];

        self.client.upsert_points(COLLECTION, points, None).await?;
        self.id += 1;

        Ok(())
    }
}