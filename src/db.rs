use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
pub struct File {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: u64,
    pub timestamp: u64,
    pub name: String, // do we need it?
    pub email: String,
    pub subject: String,
    pub message: String,
    pub op: bool,
    pub files: Vec<File>,

    pub deleted: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: u64,
    pub posts: Vec<Post>,
}

#[derive(Clone)]
pub struct DBMS {
    db_root: PathBuf,
}

impl DBMS {
    pub fn new(db: PathBuf) -> DBMS {
        fs::create_dir_all(&db.join("attachments"))
            .expect("failed to create db attachments directory");
        fs::create_dir_all(&db.join("boards")).expect("failed to create db boards directory");
        info!("Created DB");
        DBMS { db_root: db }
    }

    pub fn read_board(&self, board_id: &str) -> Option<Vec<u64>> {
        match fs::read_dir(self.db_root.join("boards").join(board_id)) {
            Ok(paths) => Some(
                paths
                    .map(|path| {
                        path.unwrap()
                            .path()
                            .file_stem()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .parse::<u64>()
                            .unwrap()
                    })
                    .collect(),
            ),
            Err(e) => {
                error!("Failed to read board /{}/: {}", board_id, e);
                None
            }
        }
    }

    pub fn read_thread(&self, board_id: &str, thread_id: u64) -> Option<Thread> {
        let filepath = &self
            .db_root
            .join("boards")
            .join(&board_id)
            .join(format!("{}.json", thread_id));
        if !filepath.exists() {
            debug!("Thread /{}/{} does not exist in db", board_id, thread_id);
            return None;
        }

        let file = fs::File::open(filepath).expect("failed to open <thread>.json");
        let thread: Thread = serde_json::from_reader(file).expect("failed to parse <thread>.json");
        debug!("Sucessfully read thread /{}/{} from db", board_id, thread_id);
        Some(thread)
    }

    pub fn write_thread(&self, board_id: &str, thread: &Thread) {
        let board_dir = &self.db_root.join("boards").join(&board_id);
        fs::create_dir_all(board_dir).expect("failed to create board directory");

        let filepath = &board_dir.join(format!("{}.json", &thread.id));
        fs::write(filepath, serde_json::to_string_pretty(&thread).unwrap())
            .expect("failed to write <thread>.json");
        debug!("Successfully wrote thread /{}/{} to db", board_id, thread.id);
    }

    pub fn read_attachment(&self, attachment_id: &str) -> Option<Vec<u8>> {
        let filepath = &self.db_root.join("attachments").join(&attachment_id);
        if !filepath.exists() {
            debug!("Attachment {} does not exist in db", attachment_id);
            return None;
        }

        let data: Vec<u8> = fs::read(filepath).expect("failed to read attachment file");
        debug!("Sucessfully read attachment {} from db", attachment_id);
        Some(data)
    }

    // TODO: prevent data race (very unrealistic but still)
    // TODO: prevent collisions (very unrealistic)
    pub fn write_attachment(&self, data: &[u8], extension: &str) -> Result<String, ()> {
        let hash = Sha256::new().chain_update(data).finalize().to_vec();
        let attachment_id = format!("{}.{}", hex::encode(hash), extension);

        let filepath = &self.db_root.join("attachments").join(&attachment_id);
        if filepath.exists() {
            debug!("Attachment {} already in db", attachment_id);
            return Ok(attachment_id);
        }

        let res = fs::write(filepath, data);
        return if let Ok(..) = res {
            debug!("Successfully wrote attachment {} to db", attachment_id);
            Ok(attachment_id)
        } else {
            error!("Failed to write attachment {} to db", attachment_id);
            Err(())
        };
    }
}
