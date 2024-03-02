use std::path::PathBuf;

use chrono::NaiveDate;
use tracing::error;

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct ChatModel {
    pub role: String,
    pub content: String,
    pub hash: String,
    pub embedding: Vec<f32>,
}
pub struct FsMessageRepo {
    memory: std::collections::HashMap<(String, String), ChatModel>, // Update HashMap key to include user
}

pub trait MessageRepo: Send + Sync {
    fn save_chat(&mut self, date: NaiveDate, user: String, chat: ChatModel) -> ChatModel;
    fn get_chat(&mut self, user: String, id: String) -> Result<ChatModel, ()>; // Add user parameter
    fn embeddings_search_for_user(
        &self,
        user: String,
        query_vector: Vec<f32>,
    ) -> Vec<(f32, ChatModel)>;
    fn get_all_for_user(&self, user: String) -> Vec<ChatModel>;
}

impl FsMessageRepo {
    pub fn new() -> FsMessageRepo {
        FsMessageRepo {
            memory: std::collections::HashMap::new(),
        }
    }

    fn get_all_for_user(&self, user: String) -> Vec<ChatModel> {
        let mut chats = vec![];
        for ((_, u), chat) in &self.memory {
            if u == &user {
                chats.push(chat.clone());
            }
        }
        chats
    }
}

fn cosine_similarity(v1: &Vec<f32>, v2: &Vec<f32>) -> f32 {
    let dot_product = v1.iter().zip(v2).map(|(a, b)| a * b).sum::<f32>();
    let magnitude_v1 = (v1.iter().map(|a| a.powi(2)).sum::<f32>()).sqrt();
    let magnitude_v2 = (v2.iter().map(|a| a.powi(2)).sum::<f32>()).sqrt();
    let magnitude_product = magnitude_v1 * magnitude_v2;
    dot_product / magnitude_product
}

fn get_root_path(user: String) -> std::path::PathBuf {
    let dir = match std::env::var("MESSAGE_STORAGE_PATH") {
        Ok(val) => std::path::PathBuf::from(val),
        Err(_) => dirs::data_local_dir().unwrap(),
    };

    let path = dir.join("muninn").join(user.clone());
    path
}
fn get_path_for_date(user: String, date: NaiveDate) -> std::path::PathBuf {
    let path = get_root_path(user.clone()).join(format!("{}", date.format("%Y-%m-%d")));
    path
}

fn get_from_fs(path: PathBuf) -> Vec<ChatModel> {
    let chats: Vec<ChatModel> = match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap(),
        Err(_) => vec![],
    };
    chats
}

impl MessageRepo for FsMessageRepo {
    fn save_chat(&mut self, date: NaiveDate, user: String, chat: ChatModel) -> ChatModel {
        let key = (chat.hash.clone(), user.clone());
        self.memory.insert(key, chat.clone());

        // let todays_date = chrono::Local::now().date_naive();
        let path = get_path_for_date(user.clone(), date).join("messages.json");
        // create directory if it does not exist
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut chats = get_from_fs(path.clone());

        // append chat to file if it exists or create a new file
        chats.push(chat.clone());
        let serialized = serde_json::to_string(&chats).unwrap();

        match std::fs::write(&path, serialized) {
            Ok(_) => (),
            Err(e) => {
                error!("Error writing to file: {}", e)
            }
        }
        chat
    }

    fn get_chat(&mut self, user: String, id: String) -> Result<ChatModel, ()> {
        let key = (id, user.clone()); // Create key using id and user
        let path = get_path_for_date(user.clone(), chrono::Local::now().date_naive())
            .join("messages.json");
        match self.memory.get(&key) {
            Some(chat) => Ok(chat.clone()),
            None => {
                let chats = get_from_fs(path);
                // put these in memory
                for chat in chats {
                    let key = (chat.hash.clone(), user.clone());
                    self.memory.insert(key, chat.clone());
                }
                match self.memory.get(&key) {
                    Some(chat) => Ok(chat.clone()),
                    None => {
                        error!("Chat not found");
                        Err(())
                    }
                }
            }
        }
    }

    fn get_all_for_user(&self, user: String) -> Vec<ChatModel> {
        let path = get_path_for_date(user.clone(), chrono::Local::now().date_naive())
            .join("messages.json");
        let r = get_from_fs(path);
        // if r is empty then we go searching
        if r.is_empty() {
            let path = get_root_path(user.clone());
            // find all the subdirectories
            let date_folders = match std::fs::read_dir(&path) {
                Ok(val) => val,
                Err(_) => return vec![],
            };
            let date_folders = date_folders
                .map(|x| x.unwrap().path())
                .collect::<Vec<PathBuf>>();
            // order the date folders
            let date_folders = date_folders
                .iter()
                .map(|x| {
                    let date = x.file_name().unwrap().to_str().unwrap();
                    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
                    date
                })
                .collect::<Vec<NaiveDate>>();
            //get the most recent date
            let date = date_folders.iter().max().unwrap();
            let path = get_path_for_date(user.clone(), *date).join("messages.json");
            let r = get_from_fs(path);
            r
        } else {
            r
        }
    }

    fn embeddings_search_for_user(
        &self,
        user: String,
        query_vector: Vec<f32>,
    ) -> Vec<(f32, ChatModel)> {
        let chats = self.get_all_for_user(user);

        let mut ranked_chats: Vec<(f32, ChatModel)> = vec![];
        for chat in chats {
            let similarity = cosine_similarity(&chat.embedding, &query_vector);
            ranked_chats.push((similarity, chat));
        }

        ranked_chats
    }
}

/**
 * Mocking the message repo
 */

pub struct MockMessageRepo {}
impl MockMessageRepo {
    #[allow(dead_code)]
    pub fn new() -> Self {
        MockMessageRepo {}
    }
}
impl MessageRepo for MockMessageRepo {
    fn save_chat(&mut self, _date: NaiveDate, _user: String, chat: ChatModel) -> ChatModel {
        chat
    }

    fn get_all_for_user(&self, _user: String) -> Vec<ChatModel> {
        vec![]
    }

    fn get_chat(&mut self, _user: String, id: String) -> Result<ChatModel, ()> {
        Ok(ChatModel {
            role: "user".to_string(),
            content: "Hello".to_string(),
            hash: id.clone(),
            embedding: vec![0.1, 0.2, 0.3],
        })
    }

    fn embeddings_search_for_user(
        &self,
        _user: String,
        _query_vector: Vec<f32>,
    ) -> Vec<(f32, ChatModel)> {
        vec![(
            0.1,
            ChatModel {
                role: "user".to_string(),
                content: "Hello".to_string(),
                hash: "123".to_string(),
                embedding: vec![0.1, 0.2, 0.3],
            },
        )]
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;
    #[test]
    fn test_save_chat_and_get_chat() {
        let id = Uuid::new_v4().to_string();
        let chat = ChatModel {
            role: "user".to_string(),
            content: "Hello".to_string(),
            hash: id.clone(),
            embedding: vec![0.1, 0.2, 0.3],
        };
        let expected_hash = id.clone();
        let expected_role = chat.role.clone();
        let expected_content = chat.content.clone();

        let mut repo = FsMessageRepo::new();
        let todays_date = chrono::Local::now().date_naive();
        repo.save_chat(todays_date, "test_user".to_string(), chat.clone()); // Pass user parameter

        let got_chat = repo.get_chat("test_user".to_string(), id).unwrap(); // Pass user parameter
        assert_eq!(got_chat.role, expected_role);
        assert_eq!(got_chat.content, expected_content);
        assert_eq!(got_chat.hash, expected_hash);
    }

    #[test]
    fn test_get_chat_when_no_user() {
        let id = Uuid::new_v4().to_string();
        let chat = ChatModel {
            role: "user".to_string(),
            content: "Hello".to_string(),
            hash: id.clone(),
            embedding: vec![0.1, 0.2, 0.3],
        };
        let mut repo = FsMessageRepo::new();
        let today = chrono::Local::now().date_naive();
        repo.save_chat(today, "test_user".to_string(), chat.clone());

        let got_chat = repo.get_chat("test_user2".to_string(), id);

        //test that the result was an error
        assert!(got_chat.is_err());
    }

    #[test]
    fn test_get_when_there_is_no_chat() {
        let id = Uuid::new_v4().to_string();
        let chat = ChatModel {
            role: "user".to_string(),
            content: "Hello".to_string(),
            hash: id.clone(),
            embedding: vec![0.1, 0.2, 0.3],
        };
        let mut repo = FsMessageRepo::new();
        let today = chrono::Local::now().date_naive();
        repo.save_chat(today, "test_user".to_string(), chat.clone());

        let got_chat = repo.get_chat("test_user".to_string(), uuid::Uuid::new_v4().to_string());

        //test that the result was an error
        assert!(got_chat.is_err());
    }

    #[test]
    fn test_embeddings_search_for_user() {
        let id = Uuid::new_v4().to_string();
        let chat = ChatModel {
            role: "user".to_string(),
            content: "Hello".to_string(),
            hash: id.clone(),
            embedding: vec![0.1, 0.2, 0.3],
        };
        let mut repo = FsMessageRepo::new();
        let today = chrono::Local::now().date_naive();
        repo.save_chat(today, "test_user".to_string(), chat.clone());

        let query_vector = vec![0.1, 0.2, 0.3];
        let results = repo.embeddings_search_for_user("test_user".to_string(), query_vector);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_get_all_for_user() {
        let user = "test_user2".to_string();

        // lets add some old date subdirectories
        let date = chrono::Local::now().date_naive() - chrono::Duration::days(2);
        let path = get_path_for_date(user.clone(), date);
        let _ = std::fs::create_dir_all(path);
        let date = chrono::Local::now().date_naive() - chrono::Duration::days(5);
        let path = get_path_for_date(user.clone(), date);
        let _ = std::fs::create_dir_all(path);
        // add messages for this date in particular
        let chat = ChatModel {
            role: "user".to_string(),
            content: "Hello".to_string(),
            hash: Uuid::new_v4().to_string(),
            embedding: vec![0.1, 0.2, 0.3],
        };
        let mut repo = FsMessageRepo::new();
        repo.save_chat(date, user.clone(), chat.clone());

        // delete the folder for today
        let path = get_path_for_date(user.clone(), chrono::Local::now().date_naive());
        let _ = std::fs::remove_dir_all(path);

        // set up a special user folder to only have subfolders
        // with dates in the past
        let _ = get_root_path(user.clone());

        // get chats should contain the model we addaed even though it
        // was for a date in the past
        let chats = repo.get_all_for_user(user.clone());
        assert_eq!(chats.len(), 1);
        assert_eq!(chats[0].content, "Hello");
        assert_eq!(chats[0].role, "user");
    }
}
