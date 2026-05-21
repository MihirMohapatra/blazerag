use anyhow::Context;
use qdrant_client::qdrant::{
    vectors_config::Config, CreateCollection, Distance, PointStruct, ScoredPoint,
    SearchPointsBuilder, UpsertPointsBuilder, VectorParams, Vectors, VectorsConfig,
};
use qdrant_client::Qdrant;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub text: String,
    pub metadata: serde_json::Value,
}

pub struct Retriever {
    client: Qdrant,
    collection: String,
}

impl Retriever {
    pub async fn new(url: &str, collection: &str, dims: u64) -> anyhow::Result<Self> {
        let client = Qdrant::from_url(url)
            .build()
            .context("Failed to create Qdrant client")?;

        let collections = client.list_collections().await?;
        let exists = collections.collections.iter().any(|c| c.name == collection);

        if !exists {
            tracing::info!(
                "Creating collection '{}' with {} dimensions",
                collection,
                dims
            );
            client
                .create_collection(CreateCollection {
                    collection_name: collection.to_string(),
                    vectors_config: Some(VectorsConfig {
                        config: Some(Config::Params(VectorParams {
                            size: dims,
                            distance: Distance::Cosine.into(),
                            ..Default::default()
                        })),
                    }),
                    ..Default::default()
                })
                .await
                .context("Failed to create Qdrant collection")?;
        }

        Ok(Self {
            client,
            collection: collection.to_string(),
        })
    }

    pub async fn upsert(
        &self,
        documents: &[Document],
        embeddings: &[Vec<f32>],
    ) -> anyhow::Result<()> {
        let points: Vec<PointStruct> = documents
            .iter()
            .zip(embeddings.iter())
            .map(|(doc, emb)| {
                let vectors: Vectors = emb.clone().into();
                let mut payload = std::collections::HashMap::new();
                payload.insert("text".to_string(), doc.text.clone().into());
                payload.insert("metadata".to_string(), doc.metadata.to_string().into());
                PointStruct {
                    id: Some(doc.id.clone().into()),
                    vectors: Some(vectors),
                    payload,
                }
            })
            .collect();

        self.client
            .upsert_points(UpsertPointsBuilder::new(self.collection.clone(), points))
            .await?;

        Ok(())
    }

    pub async fn search(
        &self,
        query_vector: &[f32],
        top_k: u64,
    ) -> anyhow::Result<Vec<ScoredPoint>> {
        let results = self
            .client
            .search_points(
                SearchPointsBuilder::new(self.collection.clone(), query_vector.to_vec(), top_k)
                    .with_payload(true)
                    .with_vectors(false),
            )
            .await?;

        Ok(results.result)
    }
}
