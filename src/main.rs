mod api;
mod db;

use clap::Parser;
use log::{error, info, warn};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

pub fn vecdiff(old: Vec<u64>, new: Vec<u64>) -> (Vec<u64>, Vec<u64>) {
    let o: HashSet<u64> = old.iter().cloned().collect();
    let n: HashSet<u64> = new.iter().cloned().collect();

    let (deleted, added) = (&o - &n, &n - &o);
    let deleted: Vec<u64> = deleted.iter().cloned().collect();
    let added: Vec<u64> = added.iter().cloned().collect();
    (deleted, added)
}

async fn dump_board(db: db::DBMS, board: String) {
    let api = api::API::new();

    let fetched = api
        .fetch_catalogue(board.as_str())
        .await
        .expect("failed to fetch board");

    let threads: Vec<u64> = fetched
        .threads
        .iter()
        .map(|thread| thread.num /*.parse::<u64>().unwrap()*/)
        .collect();

    info!("Fetched {} catalogue with {} posts", board, threads.len());

    let tasks: Vec<_> = threads
        .into_iter()
        .map(|thread| {
            let db = db.clone();
            let board = board.clone();
            tokio::spawn(async move {
                let success = dump_thread(db, board.as_str(), thread.clone()).await;
                if !success {
                    error!("Failed to dump thread {} {}", board, thread);
                }
            })
        })
        .collect();

    for task in tasks {
        match task.await {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to dump thread on board {}: {}", board, e);
            }
        }
    }
}

// TODO: parse posts' messages
async fn dump_thread(db: db::DBMS, board: &str, thread: u64) -> bool {
    let api = api::API::new();

    let fetched = api.fetch_thread(board, thread).await;

    if let None = fetched {
        error!("Failed to fetch thread {} {}", board, thread);
        return false;
    }
    let fetched = fetched.unwrap();

    let (discovered, loaded) = match db.read_thread(board, thread) {
        None => (
            true,
            db::Thread {
                id: thread,
                posts: vec![],
            },
        ),
        Some(loaded) => (false, loaded),
    };

    if discovered {
        info!(
            "Discovered new thread {} {} with {} posts",
            board,
            thread,
            fetched.threads.thread.posts.len()
        );
    }

    // merge

    let (deleted, added) = vecdiff(
        loaded.posts.iter().map(|post| post.id).collect(),
        fetched
            .threads
            .thread
            .posts
            .iter()
            .map(|post| post.num)
            .collect(),
    );

    if !discovered && deleted.is_empty() && added.is_empty() {
        info!("Nothing new in thread {} {}", board, thread);
        return true;
    }

    if !discovered && !added.is_empty() {
        info!("Thread {} {} - {} new posts", board, thread, added.len());
    }
    if !discovered && !deleted.is_empty() {
        warn!(
            "Thread {} {} - {} deleted posts",
            board,
            thread,
            deleted.len(),
        );
    }

    async fn dump_file(
        db: &db::DBMS,
        api: &api::API,
        file: &api::ThreadThreadsThreadPostFile,
    ) -> Option<db::File> {
        let attachment = api.fetch_attachment(file.path.as_str()).await;
        if let None = attachment {
            return None;
        }
        let attachment = attachment.unwrap();

        let extension = Path::new(&file.path)
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("");

        let attachment_id = db
            .write_attachment(attachment.as_slice(), extension)
            .unwrap_or(String::from(""));

        Some(db::File {
            id: attachment_id,
            name: file.name.clone(),
        })
    }

    async fn post2post(
        db: &db::DBMS,
        api: &api::API,
        post: &api::ThreadThreadsThreadPost,
    ) -> db::Post {
        db::Post {
            id: post.num,
            timestamp: post.timestamp,
            name: post.name.clone(),
            email: post.email.clone(),
            subject: post.subject.clone(),
            message: post.comment.clone(),
            op: match post.op {
                0 => false,
                1 => true,
                _ => panic!("unknown bool value"),
            },
            files: {
                let mut files = vec![];
                if let Some(post_files) = &post.files {
                    for file in post_files {
                        if let Some(fetched) = dump_file(db, api, &file).await {
                            files.push(fetched);
                        } else {
                            error!("Failed to dump file {}", file.path);
                        }
                    }
                }
                files
            },
            deleted: false,
        }
    }

    let mut old_posts = loaded.posts;
    for mut post in &mut old_posts {
        if deleted.contains(&post.id) {
            post.deleted = true;
        }
    }

    let mut new_posts = vec![];
    for post in fetched.threads.thread.posts {
        if added.contains(&post.num) {
            new_posts.push(post2post(&db, &api, &post).await);
        }
    }

    old_posts.append(&mut new_posts);

    let thread = db::Thread {
        id: loaded.id,
        posts: old_posts,
    };

    db.write_thread(board, &thread);
    info!("Dumped thread {} {}", board, thread.id);
    true
}

async fn monitor_board(db: db::DBMS, board: String, interval: Duration) {
    loop {
        dump_board(db.clone(), board.clone()).await;
        info!("Dumped board {}", board);
        // TODO: stop on error
        thread::sleep(interval);
    }
    info!("Finished monitoring board {}", board);
}

async fn monitor_thread(db: db::DBMS, board: &str, thread: u64, interval: Duration) {
    loop {
        let success = dump_thread(db.clone(), board, thread).await;

        if success {
            info!("Dumped thread {} {}", board, thread);
        } else {
            info!("Failed to dump thread {} {}", board, thread);
            break;
        }

        thread::sleep(interval);
    }
    info!("Finished monitoring thread {} {}", board, thread);
}

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    command: Command,

    #[clap(long, default_value = ".")]
    db: String,

    #[clap(long)]
    board: String,

    #[clap(long)]
    thread: Option<u64>,
}

#[derive(clap::Subcommand)]
enum Command {
    Monitor {
        #[clap(long, default_value = "0")]
        interval: u64,
    },
    Dump,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Stdout)
        .init();

    let args = Args::parse();

    let db = db::DBMS::new(PathBuf::from(args.db));

    match &args.command {
        Command::Monitor { interval } => match args.thread {
            None => {
                monitor_board(db, args.board, Duration::from_secs(*interval)).await;
            }
            Some(thread) => {
                monitor_thread(
                    db,
                    args.board.as_str(),
                    thread,
                    Duration::from_secs(*interval),
                )
                .await;
            }
        },
        Command::Dump => match args.thread {
            None => {
                dump_board(db, args.board).await;
            }
            Some(thread) => {
                dump_thread(db, args.board.as_str(), thread).await;
            }
        },
    }
}
