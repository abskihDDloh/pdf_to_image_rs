use crate::check_path::is_valid_file;
use crate::get_thread_id::get_thread_id_number;
use crate::set_workers_limit::get_sub_workers_limit;

use chrono::{self, Utc};
use log::{error, info, log_enabled, warn, Level};
use pdf::any::AnySync;
use pdf::backend::Backend;
use pdf::enc::StreamFilter;
use pdf::file::Cache;
use pdf::file::File as PdfFile;
use pdf::file::Log;
use pdf::primitive::Name;
use pdf::{error::PdfError, file::FileOptions, object::*};
use regex::{Captures, Regex};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread;
use threadpool::ThreadPool;

///PDFファイルから画像を取得する。
/// # Arguments
/// * `pdf_file_path` - PDFファイルのパス
/// # Returns
/// * 1:ページ取得失敗もしくはページ内画像取得失敗
/// * 成功時は0を返す。
/// * 失敗時はエラーコードを返す。
/// * 20:PDFファイルのフルパス取得失敗
/// * 21:ディレクトリ作成失敗
/// * 22:PDFファイルオープン失敗
///
pub fn get_images(pdf_file_path: &Path) -> u32 {
    let mut return_value: u32 = 0;
    let start_time: i64 = Utc::now().timestamp_micros();
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
            return 20;
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
                return 21;
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
            return 22;
        }
    });

    //let pool = ThreadPool::new(1);
    let pool = ThreadPool::new(get_sub_workers_limit(50.0));
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
                if return_value == 0 {
                    return_value = 1;
                    if log::log_enabled!(log::Level::Debug) {
                        info!("RETURN VALUE : {}", return_value);
                    }
                }
                continue;
            }
        };

        // 以下のように参照を作成してクロージャに渡す
        let file_ref = Arc::clone(&file);
        let image_hash_list_ref = Arc::clone(&image_hash_list);
        let dest_dir_path_ref = Arc::clone(&dest_dir_path);
        let pdf_parh_string: String = pdf_path.display().to_string();

        //get_images_from_page()を使ってスレッドを生成して画像を取得する。
        // スレッドプールにタスクを追加
        pool.execute(move || {
            match get_images_from_page(
                &page,
                file_ref,
                image_hash_list_ref,
                dest_dir_path_ref,
                &my_thread_id,
                start_time,
                page_counter,
            ) {
                Ok(result) => {
                    if log_enabled!(Level::Debug) {
                        info!(
                            "PAGE PROCESS COMPLETE. PAGE: {} FILE : {} RESULT : {}",
                            page_counter, pdf_parh_string, result
                        )
                    }
                }
                Err(e) => {
                    error!(
                        "PAGE PROCESS ERROR. PAGE: {} FILE : {} ERR : {}",
                        page_counter, pdf_parh_string, e
                    );
                    if return_value == 0 {
                        return_value = 1;
                        if log::log_enabled!(log::Level::Debug) {
                            info!("START RETURN VALUE : {}", return_value);
                        }
                    }
                }
            }
        });
    }
    // 全てのタスクが終了するのを待つ
    pool.join();
    let end_time: i64 = Utc::now().timestamp_micros();
    let elapsed_time: i64 = end_time - start_time;
    info!(
        "ALL THREADS FINISHED. FILE: {} ELAPSED_TIME : {}",
        pdf_path.display(),
        elapsed_time
    );
    return_value
}

///PDFファイルのページから画像を取得する。
/// # Arguments
/// * `page` - PDFファイルのページ
/// * `file` - PDFファイル
/// * `images_kvs` - 画像データのハッシュセット(スレッド感で共有するためArc<RwLock<HashSet<Arc<[u8]>>>>)
/// * `dest_dir_path` - 画像ファイルの保存先ディレクトリ
/// * `parent_thread_id` - 親スレッドのID(保存する画像のファイル名に使用するため)
/// * `unixtime_val` - 現在時刻のUNIXTIME(保存する画像のファイル名に使用するため)
/// * `page_count` - PDFのページ番号(保存する画像のファイル名に使用するため)
/// # Returns
/// * 成功時は0を返す。
/// * 失敗時はエラーコードを返す。
/// * (0以外のエラーコードには、少なくとも一部の画像取得に失敗した以外の意味はない。失敗の詳細は出力されるロクに出力される。)
fn get_images_from_page<T, K, Y, L>(
    page: &PageRc,
    file: Arc<PdfFile<T, K, Y, L>>,
    images_kvs: Arc<RwLock<HashSet<Arc<[u8]>>>>,
    dest_dir_path: Arc<PathBuf>,
    parent_thread_id: &std::thread::ThreadId,
    unixtime_val: i64,
    page_count: u64,
) -> Result<u32, PdfError>
where
    T: Backend,
    K: Cache<std::result::Result<AnySync, Arc<PdfError>>>,
    Y: Cache<std::result::Result<Arc<[u8]>, Arc<PdfError>>>,
    L: Log,
{
    let mut return_value: u32 = 0;
    let re = Regex::new(r"\d+").unwrap();
    let my_thread_id: std::thread::ThreadId = thread::current().id();

    let mut images: HashMap<Name, RcRef<XObject>> = HashMap::new();
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

    for (name, &r) in resources.xobjects.iter() {
        let object = resolver.get(r).unwrap();
        if matches!(*object, pdf::object::XObject::Image(_)) {
            if log_enabled!(Level::Debug) {
                log::info!(
                    "XObject_Name: {} DEST_PATH : {} PAGE: {}",
                    name,
                    dest_dir_path.display(),
                    page_count
                );
            }
            images.insert(name.clone(), object.clone());
        }
    }
    if log_enabled!(Level::Debug) {
        log::info!(
            "THIS PAGE IMAGES COUNT. PAGE : {} IMAGES: {}",
            page_count,
            images.len()
        );
    }

    let mut image_count: i64 = 0;

    for o in images.iter() {
        image_count += 1;

        let img = match **o.1 {
            XObject::Image(ref im) => im,
            _ => {
                continue;
            }
        };
        let (data, filter) = img.raw_image_data(&resolver)?;
        let ext = match filter {
            Some(StreamFilter::DCTDecode(_)) => "jpg",
            Some(StreamFilter::JBIG2Decode(_)) => "jbig2",
            Some(StreamFilter::JPXDecode) => "jp2k",
            _ => {
                if log_enabled!(Level::Warn) {
                    let hex_dump: Vec<String> =
                        data.iter().take(8).map(|b| format!("{:02x}", b)).collect();
                    warn!(
                    "UNSUPPORTED IMAGE FORMAT. TOP_8 : {} OBJECT_NAME: {} DEST_PATH : {} PAGE: {} IMAGE_COUNT : {}",
                    hex_dump.join("_"),
                    o.0,
                    dest_dir_path.display(),
                    page_count,
                    image_count
                );
                }
                return_value += 1;
                continue;
            }
        };

        //PDFファイル内の同じ画像はスキップする。
        {
            let read_set = images_kvs.read().unwrap();
            if read_set.contains(&data) {
                if log_enabled!(Level::Debug) {
                    info!(
                        "IMAGE FILE ALREADY EXISTS. OBJECT_NAME: {} DEST_PATH : {} PAGE: {} IMAGE_COUNT : {}",
                        o.0,
                        dest_dir_path.display(),
                        page_count,
                        image_count
                    );
                }
                continue;
            }
        }
        {
            //まだ処理されていない画像であればHashSetの書き込みロックを取得して再確認する。
            let mut write_set = images_kvs.write().unwrap();
            if write_set.contains(&data) {
                if log_enabled!(Level::Debug) {
                    info!(
                        "IMAGE FILE ALREADY EXISTS. OBJECT_NAME: {} DEST_PATH : {} PAGE: {} IMAGE_COUNT : {}",
                        o.0,
                        dest_dir_path.display(),
                        page_count,
                        image_count
                    );
                }
                continue;
            }

            //埋め込みオブジェクト名の数字を6桁に変換する。
            let converted_embbeded_object_name: std::borrow::Cow<str> =
                re.replace_all(o.0, |caps: &Captures| {
                    let num: u32 = (caps[0]).parse().unwrap();
                    format!("{:06}", num)
                });

            let save_path_str = format!(
                "{}/image_{}_{}_{:06}_{:06}_{:06}_{:06}.{}",
                dest_dir_path.display(),
                unixtime_val,
                converted_embbeded_object_name,
                image_count,
                page_count,
                get_thread_id_number(parent_thread_id),
                get_thread_id_number(&my_thread_id),
                ext
            );

            //画像ファイルを作成する。
            let mut output = match File::create(&save_path_str) {
                Ok(file) => file,
                Err(e) => {
                    warn!(
                    "COULD NOT CREATE IMAGE FILE. OBJECT_NAME: {} DEST_PATH : {} PAGE: {} IMAGE_COUNT : {} ERR: {}",
                    o.0,save_path_str, page_count, image_count, e
                );
                    return_value += 1;
                    continue;
                }
            };

            //画像ファイルの書き込みを行う。
            match output.write_all(&data) {
                Ok(_) => {
                    if log_enabled!(Level::Debug) {
                        info!(
                        "IMAGE FILE WRITTEN. OBJECT_NAME: {} DEST_PATH : {} PAGE: {} IMAGE_COUNT : {}",
                        o.0,save_path_str, page_count, image_count
                    );
                    }
                }
                Err(e) => {
                    warn!(
                    "COULD NOT WRITE IMAGE FILE. OBJECT_NAME: {} DEST_PATH : {} PAGE: {} IMAGE_COUNT : {} ERR: {}",
                    o.0,save_path_str, page_count, image_count, e
                );
                    return_value += 1;
                    continue;
                }
            };
            //ファイルの書き込みに成功したらHashSetに画像データのハッシュを追加する。
            write_set.insert(data.clone());
            if log_enabled!(Level::Debug) {
                info!(
                    "NEW HASH INSERTED. OBJECT_NAME: {} DEST_PATH: {} PAGE: {} IMAGE_COUNT : {} HASHSET_LENGTH: {}",
                    o.0,
                    dest_dir_path.display(),
                    page_count,
                    image_count,
                    write_set.len()
                );
            }
        }
    }
    Ok(return_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use std::path::Path;

    fn check_files_with_extension(directory: &str, extension: &str) -> bool {
        let entries = fs::read_dir(directory).expect("Unable to read directory");
        for entry in entries {
            let entry = entry.expect("Unable to read directory entry");
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some(extension) {
                return true;
            }
        }

        false
    }

    #[test_log::test]
    ///正常なPDFファイルから画像を取得するテスト
    /// 1.正常なPDFファイルを指定して画像を取得する。
    /// 2.画像が取得できたか確認する。
    /// 3.画像が取得できた場合は画像が入っているディレクトリを削除する。
    /// 4.ディレクトリが削除されたか確認する。
    /// (画像のファイル名が正しいかどうかについてはテストしない。)
    fn test_get_images_valid_pdf() {
        let dir_str: &str = "test_pdf/correct_pdf";
        let file_name_str: &str = "aaa";
        let pdf_extension: &str = "pdf";
        let file_string: String = format!("{}/{}.{}", dir_str, file_name_str, pdf_extension);
        let pdf_file_path = Path::new(file_string.as_str());
        let result = get_images(pdf_file_path);
        assert_eq!(result, 0);
        let extension = "jpg";
        let dest_dir_string: String = format!("{}/{}", dir_str, file_name_str);
        let exists = check_files_with_extension(dest_dir_string.as_str(), extension);
        assert!(exists);
        //ディレクトリを削除する。
        if exists {
            let dest_cp = dest_dir_string.clone();
            let res = fs::remove_dir_all(dest_dir_string);
            if res.is_err() {
                panic!(
                    "COULD NOT REMOVE DIRECTORY. DIRECTORY: {} ERR: {}",
                    dest_cp,
                    res.err().unwrap()
                );
            }
            assert!(res.is_ok());
        }
    }

    #[test_log::test]
    fn test_get_images_invalid_pdf() {
        let pdf_file_path = Path::new("path/to/invalid.pdf");
        let result = get_images(pdf_file_path);
        assert_ne!(result, 0);
    }

    #[test_log::test]
    fn test_get_images_existing_directory() {
        let pdf_file_path = Path::new("test_pdf/correct_pdf");
        let result = get_images(pdf_file_path);
        assert_eq!(result, 20);
    }

    #[test_log::test]
    fn test_get_images_non_existing_directory() {
        let pdf_file_path = Path::new("path/to/non_existing_directory");
        let result = get_images(pdf_file_path);
        assert_ne!(result, 0);
    }
}
