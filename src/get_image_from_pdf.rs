use crate::check_path::is_valid_file;
use crate::set_workers_limit::get_sub_workers_limit;

use chrono::{DateTime, Utc};
use image::guess_format;
use log::info;
use log::{debug, error, warn};
use pdf::any::AnySync;
use pdf::backend::Backend;
use pdf::file::Cache;
use pdf::file::File as PdfFile;
use pdf::file::Log;
use pdf::{error::PdfError, file::FileOptions, object::*};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::File;
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

    let pool = ThreadPool::new(get_sub_workers_limit());

    let image_hash_list: Arc<RwLock<HashSet<String>>> = Arc::new(RwLock::new(HashSet::new()));

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
    images_kvs: Arc<RwLock<HashSet<String>>>,
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
        let data: Arc<[u8]> = img.image_data(&resolver)?;

        //PDFファイル内の同じ画像はスキップする。
        let hash: String = format!("{:x}", Sha256::digest(&data));

        info!("IMAGE HASH: {} DEST_PATH : {} PAGE: {} IMAGE_COUNT : {}",
        hash,
        dest_dir_path.display(),
        page_count,
        image_count);

        let read_set: std::sync::RwLockReadGuard<HashSet<String>> = images_kvs.read().unwrap();
        if read_set.contains(&hash) {
            info!("IMAGE FILE ALREADY EXISTS. HASH: {} DEST_PATH : {} PAGE: {} IMAGE_COUNT : {}",
            hash,
            dest_dir_path.display(),
            page_count,
            image_count);
            continue;
        }

        if let Ok(format) = guess_format(&data) {
            info!("IMAGE_FOUND. FORMAT: {:?} HASH: {}", format, hash);
            let mut write_set: std::sync::RwLockWriteGuard<HashSet<String>> =
                images_kvs.write().unwrap();
            let image = image::load_from_memory(&data).unwrap();
            let save_path_str = format!(
                "{}/image_{:?}_{:?}_{}_{}_{}.{}",
                dest_dir_path.display(),
                unixtime_val,
                format!("{:?}", parent_thread_id),
                page_count,
                format!("{:?}", my_thread_id),
                image_count,
                format.extensions_str()[0]
            );
            let mut output = match File::create(save_path_str) {
                Ok(file) => file,
                Err(e) => {
                    warn!(
                        "COULD NOT CREATE IMAGE FILE. DEST_PATH : {} PAGE: {} IMAGE_COUNT : {} ERR: {}",
                        dest_dir_path.display(),
                        page_count,
                        image_count,
                        e
                    );
                    continue;
                }
            };
            match image.write_to(&mut output, format) {
                Ok(_) => {
                    info!(
                        "IMAGE FILE WRITTEN. DEST_PATH : {} PAGE: {} IMAGE_COUNT : {}",
                        dest_dir_path.display(),
                        page_count,
                        image_count
                    );
                    write_set.insert(hash);
                    info!(
                        "HASHSET_LENGTH: {} DEST_PATH: {}",
                        write_set.len(),
                        dest_dir_path.display()
                    );
                }
                Err(e) => {
                    warn!(
                        "COULD NOT WRITE IMAGE FILE. DEST_PATH : {} PAGE: {} IMAGE_COUNT : {} ERR: {}",
                        dest_dir_path.display(),
                        page_count,
                        image_count,
                        e
                    );
                    continue;
                }
            };
        }
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
