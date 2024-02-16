use async_trait::async_trait;

use crate::repos::messages::ChatModel;
use std::sync::Arc;
use tokio::sync::Mutex; // Import the TryFutureExt trait

#[derive(serde::Deserialize, serde::Serialize)]
pub struct ChatRequest {
    pub role: String,
    pub content: String,
    pub hash: String,
}

#[derive(serde::Deserialize)]
pub struct SearchRequest {
    pub content: String,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct SearchResponse {
    pub role: String,
    pub content: String,
    pub hash: String,
    pub ranking: f32,
}
impl SearchResponse {
    fn from_chat_model(clone: ChatModel, ranking: f32) -> SearchResponse {
        SearchResponse {
            role: clone.role,
            content: clone.content,
            hash: clone.hash,
            ranking,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct ChatResponse {
    pub role: String,
    pub content: String,
    pub hash: String,
}

impl ChatResponse {
    #[allow(dead_code)]
    pub fn new(role: String, content: String, hash: String) -> ChatResponse {
        ChatResponse {
            role,
            content,
            hash,
        }
    }
    pub fn from_model(model: ChatModel) -> ChatResponse {
        ChatResponse {
            role: model.role,
            content: model.content,
            hash: model.hash,
        }
    }
}

#[derive(Clone)]
pub struct ChatHandlerImpl {
    pub(crate) embedding_client: Arc<Mutex<dyn crate::clients::embeddings::EmbeddingsClient>>,
    pub(crate) message_repo: Arc<Mutex<dyn crate::repos::messages::MessageRepo>>,
}

#[async_trait]
pub trait ChatHandler: Send + Sync {
    async fn save_chat(&self, chat: ChatRequest) -> Result<ChatResponse, ()>;
    async fn get_chat(&self, id: &String) -> Result<ChatResponse, ()>;
    async fn search_chat(&self, query: &String) -> Result<Vec<SearchResponse>, ()>;
}

#[async_trait]
impl ChatHandler for ChatHandlerImpl {
    async fn save_chat(&self, chat: ChatRequest) -> Result<ChatResponse, ()> {
        let embeddings_client = self.embedding_client.lock().await;
        let embeddings_result = embeddings_client.get_embeddings(chat.content.clone()).await;

        let embeddings = match embeddings_result {
            Ok(embeddings) => embeddings,
            Err(_) => return Err(()),
        };

        let cm = ChatModel {
            role: chat.role.clone(),
            content: chat.content.clone(),
            hash: chat.hash.clone(),
            embedding: embeddings.clone(),
        };

        let mut message_repo = self.message_repo.lock().await;
        let result = message_repo.save_chat("my_user".to_string(), cm.clone());
        let cr = ChatResponse::from_model(result);
        Ok(cr)
    }

    async fn get_chat(&self, id: &String) -> Result<ChatResponse, ()> {
        let chat = self
            .message_repo
            .lock()
            .await
            .get_chat("my_user".to_string(), id.clone())
            .unwrap();
        let cr = ChatResponse::from_model(chat);
        Ok(cr.clone())
    }

    async fn search_chat(&self, query: &String) -> Result<Vec<SearchResponse>, ()> {
        let repo = self.message_repo.lock().await;
        let user = "my_user".to_string();

        let embeddings_client = self.embedding_client.lock().await;
        let query_vector = embeddings_client
            .get_embeddings(query.clone())
            .await
            .unwrap();

        let founds = repo.embeddings_search_for_user(user, query_vector);
        let founds = founds
            .iter()
            .map(|(similarity, chat)| {
                SearchResponse::from_chat_model(chat.clone(), similarity.clone())
            })
            .collect();
        Ok(founds)
    }
}
