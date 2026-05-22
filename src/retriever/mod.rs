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
    collection_prefix: String,
    dims: u64,
}

impl Retriever {
    pub async fn new(url: &str, collection: &str, dims: u64) -> anyhow::Result<Self> {
        let client = Qdrant::from_url(url)
            .build()
            .context("Failed to create Qdrant client")?;

        // Validate connection by listing collections
        client
            .list_collections()
            .await
            .context("Failed to connect to Qdrant")?;

        tracing::info!(
            "Retriever initialized, collection_prefix={}, dims={}",
            collection,
            dims
        );

        Ok(Self {
            client,
            collection_prefix: collection.to_string(),
            dims,
        })
    }

    fn collection_name(&self, tenant_id: &str) -> String {
        if tenant_id.is_empty() || tenant_id == "default" {
            self.collection_prefix.clone()
        } else {
            format!("{}_{}", self.collection_prefix, tenant_id)
        }
    }

    pub async fn ensure_collection(&self, tenant_id: &str) -> anyhow::Result<()> {
        let name = self.collection_name(tenant_id);
        let collections = self.client.list_collections().await?;
        let exists = collections.collections.iter().any(|c| c.name == name);

        if !exists {
            tracing::info!(
                "Creating collection '{}' with {} dimensions",
                name,
                self.dims
            );
            self.client
                .create_collection(CreateCollection {
                    collection_name: name,
                    vectors_config: Some(VectorsConfig {
                        config: Some(Config::Params(VectorParams {
                            size: self.dims,
                            distance: Distance::Cosine.into(),
                            ..Default::default()
                        })),
                    }),
                    ..Default::default()
                })
                .await
                .context("Failed to create Qdrant collection")?;
        }

        Ok(())
    }

    pub async fn upsert(
        &self,
        tenant_id: &str,
        documents: &[Document],
        embeddings: &[Vec<f32>],
    ) -> anyhow::Result<()> {
        self.ensure_collection(tenant_id).await?;
        let collection = self.collection_name(tenant_id);

        let points: Vec<PointStruct> = documents
            .iter()
            .zip(embeddings.iter())
            .map(|(doc, emb)| {
                let vectors: Vectors = emb.clone().into();
                let mut payload = std::collections::HashMap::new();
                payload.insert("text".to_string(), doc.text.clone().into());
                payload.insert("metadata".to_string(), doc.metadata.to_string().into());
                payload.insert("tenant_id".to_string(), tenant_id.to_string().into());
                PointStruct {
                    id: Some(doc.id.clone().into()),
                    vectors: Some(vectors),
                    payload,
                }
            })
            .collect();

        self.client
            .upsert_points(UpsertPointsBuilder::new(collection, points))
            .await?;

        Ok(())
    }

    pub async fn search(
        &self,
        tenant_id: &str,
        query_vector: &[f32],
        top_k: u64,
    ) -> anyhow::Result<Vec<ScoredPoint>> {
        self.ensure_collection(tenant_id).await?;
        let collection = self.collection_name(tenant_id);

        let results = self
            .client
            .search_points(
                SearchPointsBuilder::new(collection, query_vector.to_vec(), top_k)
                    .with_payload(true)
                    .with_vectors(false),
            )
            .await?;

        Ok(results.result)
    }
}
