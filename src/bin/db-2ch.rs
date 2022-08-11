use clap::Parser;
use crawler_2ch::db;
use log::{error, info};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

async fn copy_board(from: db::DBMS, to: db::DBMS, board: &str) {
    error!("Not implemented xd");
    // TODO:
}

async fn copy_thread(from: db::DBMS, to: db::DBMS, board: &str, thread_id: u64) {
    let thread = from
        .read_thread(board, thread_id)
        .expect("failed to read thread");
    to.write_thread(board, &thread);
    info!("Copied thread to db");

    info!("Copying attachments...");
    let attachments: Vec<&str> = thread
        .posts
        .iter()
        .map(|post| -> Vec<&str> {
            post.files
                .iter()
                .map(|file| -> &str { &file.id.as_str() })
                .collect()
        })
        .flatten()
        .collect();

    for attachment_id in attachments {
        // not efficient in case of filesystem db
        let attachment = from
            .read_attachment(attachment_id)
            .expect("failed to read attachment");
        let extension = Path::new(&attachment_id)
            .extension()
            .and_then(OsStr::to_str)
            .unwrap();
        to.write_attachment(attachment.as_slice(), extension)
            .expect("Failed to write attachment");
    }
    info!("Done");
}

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    Copy {
        #[clap(long)]
        from: String,

        #[clap(long)]
        to: String,

        #[clap(long)]
        board: String,

        #[clap(long)]
        thread: Option<u64>,
    },
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env().init();

    let args = Args::parse();

    match &args.command {
        Command::Copy {
            from,
            to,
            board,
            thread,
        } => {
            let from = db::DBMS::new(PathBuf::from(from));
            let to = db::DBMS::new(PathBuf::from(to));
            match thread {
                Some(thread) => {
                    copy_thread(from, to, board, *thread).await;
                }
                None => {
                    copy_board(from, to, board).await;
                }
            }
        }
    }
}
