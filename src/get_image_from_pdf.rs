use crate::check_path::is_valid_file;
use crate::set_workers_limit::get_sub_workers_limit;

use chrono::{DateTime, Utc};
use log::{error, info, log_enabled, warn, Level};
use pdf::any::AnySync;
use pdf::backend::Backend;
use pdf::enc::StreamFilter;
use pdf::file::Cache;
use pdf::file::File as PdfFile;
use pdf::file::Log;
use pdf::{error::PdfError, file::FileOptions, object::*};
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread;
use threadpool::ThreadPool;

pub fn get_images(pdf_file_path: &Path) -> i64 {
    let dt: DateTime<Utc> = Utc::now();
    let unixtime_val: i64 = dt.timestamp_millis();
    let my_thread_id: std::thread::ThreadId = thread::current().id();

    //受け取ったファイルのパスをフルパスに変換する。
    let pdf_path = match is_valid_file(pdf_file_path) {
        Ok(path) => path,
        Err(e) => {
            error!(
                "COULD NOT GET PDF FULL PATH. FILE: {} ERR: {}",
                pdf_file_path.display(),
                e
            );
            return 10;
        }
    };

    //pdf_pathから拡張子を取り除く。
    let dest_dir_path: Arc<PathBuf> = Arc::new(pdf_path.with_extension(""));
    //dest_dir_pathの示すディレクトリが存在していない場合はディレクトリを作成する。
    if !dest_dir_path.is_dir() {
        match std::fs::create_dir_all(dest_dir_path.as_ref()) {
            Ok(_) => {}
            Err(e) => {
                error!(
                    "COULD NOT CREATE DIRECTORY. DIRECTORY: {} ERR: {}",
                    dest_dir_path.display(),
                    e
                );
                return 11;
            }
        }
    } else {
        info!(
            "DIRECTORY ALREADY EXISTS. IGNORE THIS FILE. DIRECTORY: {} FILE : {}",
            dest_dir_path.display(),
            pdf_path.display()
        );
        return 0;
    };

    //PDFファイルを開く
    let file = Arc::new(match FileOptions::cached().open(&pdf_path) {
        Ok(file) => file,
        Err(e) => {
            error!(
                "COULD NOT OPEN PDF FILE. FILE: {} ERR: {}",
                pdf_path.display(),
                e
            );
            return 12;
        }
    });

    let pool = ThreadPool::new(1);
    //let pool = ThreadPool::new(get_sub_workers_limit(50.0));
    let image_hash_list: Arc<RwLock<HashSet<Arc<[u8]>>>> = Arc::new(RwLock::new(HashSet::new()));
    let mut page_counter: u64 = 0;

    for page in file.pages() {
        page_counter += 1;
        let page: PageRc = match page {
            Ok(page) => page,
            Err(e) => {
                warn!(
                    "COULD NOT GET PAGE. IT IGNORED. PAGE: {} FILE: {} ERR: {}",
                    pdf_path.display(),
                    page_counter,
                    e
                );
                continue;
            }
        };

        // 以下のように参照を作成してクロージャに渡す
        let file_ref = Arc::clone(&file);
        let image_hash_list_ref = Arc::clone(&image_hash_list);
        let dest_dir_path_ref = Arc::clone(&dest_dir_path);

        //get_images_from_page()を使ってスレッドを生成して画像を取得する。

        // スレッドプールにタスクを追加
        pool.execute(move || {
            match get_images_from_page(
                &page,
                file_ref,
                image_hash_list_ref,
                dest_dir_path_ref,
                &my_thread_id,
                unixtime_val,
                page_counter,
            ) {
                Ok(result) => println!("Thread finished with result: {}", result),
                Err(e) => println!("Thread encountered an error: {}", e),
            }
        });
    }
    // 全てのタスクが終了するのを待つ
    pool.join();
    return 0;
}

fn get_images_from_page<T, K, Y, L>(
    page: &PageRc,
    file: Arc<PdfFile<T, K, Y, L>>,
    images_kvs: Arc<RwLock<HashSet<Arc<[u8]>>>>,
    dest_dir_path: Arc<PathBuf>,
    parent_thread_id: &std::thread::ThreadId,
    unixtime_val: i64,
    page_count: u64,
) -> Result<i64, PdfError>
where
    T: Backend,
    K: Cache<std::result::Result<AnySync, Arc<PdfError>>>,
    Y: Cache<std::result::Result<Arc<[u8]>, Arc<PdfError>>>,
    L: Log,
{
    let my_thread_id: std::thread::ThreadId = thread::current().id();

    let mut images = vec![];
    let resources: &MaybeRef<Resources> = {
        match page.resources() {
            Ok(resources) => resources,
            Err(e) => {
                error!(
                    "COULD NOT GET PAGE RESOURCES. DEST_PATH : {} PAGE: {} ERR: {}",
                    dest_dir_path.display(),
                    page_count,
                    e
                );
                return Err(e);
            }
        }
    };
    let resolver = file.resolver();

    images.extend(
        resources
            .xobjects
            .iter()
            .map(|(_name, &r)| resolver.get(r).unwrap())
            .filter(|o| matches!(**o, pdf::object::XObject::Image(_))),
    );

    log::info!(
        "THIS PAGE IMAGES COUNT. PAGE : {} IMAGES: {}",
        page_count,
        images.len()
    );

    let mut image_count: i64 = 0;

    for o in images.iter() {
        image_count += 1;

        let img = match **o {
            XObject::Image(ref im) => im,
            _ => continue,
        };
        let (data, filter) = img.raw_image_data(&resolver)?;
        let ext = match filter {
            Some(StreamFilter::DCTDecode(_)) => "jpg",
            Some(StreamFilter::JBIG2Decode(_)) => "jbig2",
            Some(StreamFilter::JPXDecode) => "jp2k",
            Some(StreamFilter::CCITTFaxDecode(_)) => "tiff",
            Some(StreamFilter::FlateDecode(_)) => "png",
            _ => {
                if log_enabled!(Level::Warn) {
                    let hex_dump: Vec<String> =
                        data.iter().take(8).map(|b| format!("{:02x}", b)).collect();
                    warn!(
                    "UNSUPPORTED IMAGE FORMAT. TOP_8 : {} DEST_PATH : {} PAGE: {} IMAGE_COUNT : {}",
                    hex_dump.join("_"),
                    dest_dir_path.display(),
                    page_count,
                    image_count
                );
                }
                continue;
            }
        };

        //PDFファイル内の同じ画像はスキップする。
        {
            let read_set = images_kvs.read().unwrap();
            if read_set.contains(&data) {
                if log_enabled!(Level::Debug) {
                    info!(
                        "IMAGE FILE ALREADY EXISTS. DEST_PATH : {} PAGE: {} IMAGE_COUNT : {}",
                        dest_dir_path.display(),
                        page_count,
                        image_count
                    );
                }
                continue;
            }
        }
        {
            let mut write_set = images_kvs.write().unwrap();
            if write_set.contains(&data) {
                if log_enabled!(Level::Debug) {
                    info!(
                        "IMAGE FILE ALREADY EXISTS. DEST_PATH : {} PAGE: {} IMAGE_COUNT : {}",
                        dest_dir_path.display(),
                        page_count,
                        image_count
                    );
                }
                continue;
            }
            write_set.insert(data.clone());
            if log_enabled!(Level::Info) {
                info!(
                    "NEW HASH INSERTED. HASHSET_LENGTH: {} DEST_PATH: {} PAGE: {} IMAGE_COUNT : {}",
                    write_set.len(),
                    dest_dir_path.display(),
                    page_count,
                    image_count
                );
            }
        }
        
        let save_path_str = format!(
            "{}/image_{}_{:06}_{:06}_{:?}_{:?}.{}",
            dest_dir_path.display(),
            unixtime_val,
            image_count,
            page_count,
            parent_thread_id,
            my_thread_id,
            ext
        );

        let mut output = match File::create(&save_path_str) {
            Ok(file) => file,
            Err(e) => {
                warn!(
                    "COULD NOT CREATE IMAGE FILE. DEST_PATH : {} PAGE: {} IMAGE_COUNT : {} ERR: {}",
                    save_path_str, page_count, image_count, e
                );
                continue;
            }
        };
        match output.write(&data) {
            Ok(_) => {
                info!(
                    "IMAGE FILE WRITTEN. DEST_PATH : {} PAGE: {} IMAGE_COUNT : {}",
                    save_path_str,
                    page_count,
                    image_count
                );
            }
            Err(e) => {
                warn!(
                    "COULD NOT WRITE IMAGE FILE. DEST_PATH : {} PAGE: {} IMAGE_COUNT : {} ERR: {}",
                    save_path_str,
                    page_count,
                    image_count,
                    e
                );
                continue;
            }
        };
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_images() {
        env::set_var("RUST_LOG", "info");
        env_logger::init();
        let pdf_file_path = Path::new("test_pdf/correct_pdf/aaa.pdf");
        let result = get_images(pdf_file_path);
        assert_eq!(result, 0);
    }
}
