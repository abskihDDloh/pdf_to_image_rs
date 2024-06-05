mod conf_logger;
use crate::conf_logger::init_logger;
mod check_path;
use crate::check_path::is_valid_directory;
mod seek_pdf;
use crate::seek_pdf::seek_pdf_file;

use clap::Parser;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    pdfdir: String,
}

fn start(directory_path: &Path) -> i64 {
    let ret=is_valid_directory(directory_path).unwrap();
    let pdf_files: Vec<std::path::PathBuf> = seek_pdf_file(directory_path).unwrap();
    return 0;
}

fn main() {
    init_logger();
    let args = Args::parse();
    let pdf_dir_str: String = args.pdfdir;
    let path = Path::new(pdf_dir_str.as_str());
    start(path);
}
