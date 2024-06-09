mod check_path;
use crate::check_path::is_valid_directory;
mod seek_pdf;
use crate::seek_pdf::seek_pdf_file;
mod get_image_from_pdf;
mod get_thread_id;
mod set_workers_limit;

use chrono::{self, Utc};
use clap::Parser;
use env_logger;
use get_image_from_pdf::get_images;
use log::{error, info};
use set_workers_limit::get_main_workers_limit;
use std::env;
use std::path::Path;
use std::sync::Arc;
use threadpool::ThreadPool;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short = 's', long = "pdfdir", help = "pdfファイルが格納されているディレクトリのパスを指定します。")]
    pdfdir: String,

    #[arg(short = 'd', long = "debug", help = "でバッグモードを有効にします。")]
    debug: bool,
}

fn start(directory_path: &Path) -> i64 {
    let src_dir_res: Result<std::path::PathBuf, std::io::Error> =
        is_valid_directory(directory_path);
    let src_dir: std::path::PathBuf;
    match src_dir_res {
        Ok(path) => {
            src_dir = path;
        }
        Err(e) => {
            error!(
                "INVALID DIRECTORY PATH. PATH : {:?} ERR: {}",
                directory_path, e
            );
            return 10;
        }
    }
    let _pdf_files_res: Result<Vec<std::path::PathBuf>, std::io::Error> =
        seek_pdf_file(src_dir.as_path());
    let _pdf_files: Vec<std::path::PathBuf>;
    match _pdf_files_res {
        Ok(files) => {
            _pdf_files = files;
        }
        Err(e) => {
            error!("ERROR OCCURED WHILE SEEKING PDF FILES. ERR: {}", e);
            return 11;
        }
    }

    let _pool = ThreadPool::new(get_main_workers_limit());
    for file in _pdf_files {
        let _handle = _pool.execute(move || {
            let file_path = file.as_path();
            let file_path_arc = Arc::new(file_path);
            let file_path_clone = file_path_arc.clone();
            let result: i64 = get_images(file_path_clone);
            match result {
                0 => info!("PDF FILE PROCESS COMPLETE. FILE : {:?}", file_path),
                _ => error!("PDF FILEeeee PROCESS ERROR. FILE : {:?} RESULT : {}", file_path, result),
            }
            ()
        });
    }
    // 全てのタスクが終了するのを待つ
    _pool.join();
    return 0;
}

fn main() {
    let args = Args::parse();
    if args.debug {
        env::set_var("RUST_LOG", "debug");
    } else {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    info!("START PDF TO IMAGE CONVERTER.");
    let start_time: i64 = Utc::now().timestamp_micros();
    let pdf_dir_str: String = args.pdfdir;
    let path = Path::new(pdf_dir_str.as_str());
    let return_value = start(path);
    let end_time: i64 = Utc::now().timestamp_micros();
    let elapsed_time: i64 = end_time - start_time;
    info!("END PDF TO IMAGE CONVERTER. ELAPSED_TIME : {}", elapsed_time);
    std::process::exit(return_value as i32);
}
