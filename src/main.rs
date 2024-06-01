

use env_logger;
#[warn(unused_imports)]
use log::{error, warn, info, debug};
use std::env;

use std::path::Path;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    pdfdir: String,
}

fn init_logger(){
    env::set_var("RUST_LOG", "info");
    env_logger::init();
}

fn start(directory_path_str: String) -> i64{
    let path = Path::new(directory_path_str.as_str());
    if !path.is_dir(){
        error!("{} is not a directory", directory_path_str);
        return 10;
    }
    info!("{} is a directory", directory_path_str);
    return 0
}

fn main() {
    init_logger();
    let args = Args::parse();
    let pdf_dir_str: String = args.pdfdir;
    start(pdf_dir_str);
}
    

#[test]
fn test_start(){
    init_logger();
    assert_eq!(start("test".to_string()), 10);
    assert_eq!(start("test_pdf".to_string()), 0);
}