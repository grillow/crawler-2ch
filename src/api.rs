use log::{debug, error};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize)]
pub struct CatalogueBoard {
    pub id: String,
    pub name: String,
    // TODO: other fields
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CatalogueThread {
    pub num: u64,
    // TODO: other fields
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Catalogue {
    pub threads: Vec<CatalogueThread>,
    // TODO: other fields
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ThreadThreadsThreadPostFile {
    pub path: String,
    pub name: String,
    // TODO: other fields
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ThreadThreadsThreadPost {
    pub num: u64,
    pub comment: String,
    pub timestamp: u64,
    pub subject: String,
    pub email: String,
    pub name: String,
    pub op: u8,
    pub files: Option<Vec<ThreadThreadsThreadPostFile>>,
    // TODO: other fields
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ThreadThreadsThread {
    pub posts: Vec<ThreadThreadsThreadPost>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ThreadThreads {
    #[serde(rename = "0")]
    pub thread: ThreadThreadsThread,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Thread {
    pub threads: ThreadThreads,
    // TODO: other fields
}

pub struct API {
    client: reqwest::Client,
}

impl API {
    const BASE_PATH: &'static str = "https://2ch.hk";

    pub fn new() -> API {
        let mut headers = HeaderMap::new();
        // TODO: read from file
        headers.insert(
            reqwest::header::CONNECTION,
            HeaderValue::try_from("keep-alive").unwrap(),
        );

        API {
            client: reqwest::ClientBuilder::new()
                .default_headers(headers)
                .http2_keep_alive_while_idle(true)
                .http2_keep_alive_timeout(Duration::from_secs(300))
                .build()
                .expect("failed to build reqwest client"),
        }
    }

    // TODO: better error handling

    pub async fn fetch_catalogue(&self, board_id: &str) -> Option<Catalogue> {
        let response = self
            .client
            .get(format!("{}/{}/catalog.json", API::BASE_PATH, board_id))
            .send()
            .await
            .unwrap();

        if response.status() != reqwest::StatusCode::OK {
            error!(
                "Failed to fetch catalogue {}: {}",
                board_id,
                response.status()
            );
            return None;
        }

        return if let Ok(text) = response.text().await {
            let catalogue: Catalogue =
                serde_json::from_str(&text).expect("failed to parse catalogue json");
            debug!("Successfully fetched catalogue {}", board_id);
            Some(catalogue)
        } else {
            error!("Failed to get catalogue body {}", board_id);
            None
        };
    }

    pub async fn fetch_thread(&self, board_id: &str, thread_id: u64) -> Option<Thread> {
        let response = self
            .client
            .get(format!(
                "{}/{}/res/{}.json",
                API::BASE_PATH,
                board_id,
                thread_id
            ))
            .send()
            .await;

        if let Err(err) = response {
            error!(
                "Failed to fetch thread /{}/{}: {}",
                board_id, thread_id, err
            );
            return None;
        }
        let response = response.unwrap();

        if response.status() != reqwest::StatusCode::OK {
            error!(
                "Failed to fetch thread /{}/{}: {}",
                board_id,
                thread_id,
                response.status()
            );
            return None;
        }

        return if let Ok(text) = response.text().await {
            let thread: Thread = serde_json::from_str(&text).expect("failed to parse thread json");
            debug!("Successfully fetched thread /{}/{}", board_id, thread_id);
            Some(thread)
        } else {
            error!("Failed to get thread body /{}/{}", board_id, thread_id);
            None
        };
    }

    pub async fn fetch_attachment(&self, path: &str) -> Option<Vec<u8>> {
        let response = self
            .client
            .get(format!("{}/{}", API::BASE_PATH, path))
            .send()
            .await
            .unwrap();

        if response.status() != reqwest::StatusCode::OK {
            error!("Failed to fetch attachment {}: {}", path, response.status());
            return None;
        }

        return if let Ok(bytes) = response.bytes().await {
            debug!("Successfully fetched attachment {}", path);
            Some(bytes.to_vec())
        } else {
            error!("Failed to get attachment body {}", path);
            None
        };
    }
}
