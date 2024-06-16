use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::check_path::is_valid_directory;

/// # 概要
/// この関数は指定されたディレクトリ内のPDFファイルを探し、そのパスのリストを返します。
///
/// # 引数
/// * `directory_path`: PDFファイルを探すディレクトリのパスを指定します。
///
/// # 戻り値
/// PDFファイルが見つかった場合はそのパスのリストを返します。
/// PDFファイルが見つからなかった場合は空のリストを返します。
/// ディレクトリが無効な場合はエラーを返します。
///
/// # 例
/// ```
/// let result = seek_pdf_file(Path::new("/path/to/directory"));
/// match result {
///     Ok(paths) => for path in paths {
///         println!("Found PDF at: {}", path.display());
///     },
///     Err(e) => println!("An error occurred: {}", e),
/// }
/// ```
///
pub fn seek_pdf_file(directory_path: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let src_dir: std::path::PathBuf = match is_valid_directory(directory_path) {
        Ok(path) => path,
        Err(e) => return Err(e),
    };
    let mut pdf_files = Vec::new();
    for entry in fs::read_dir(src_dir.as_path())? {
        let entry: fs::DirEntry = entry?;
        let path: PathBuf = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            pdf_files.push(path);
        }
    }
    Ok(pdf_files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seek_pdf_file_empty_directory() {
        let dir_str: &str = "test_pdf/empty_dir";
        let dmpty_dir_path = Path::new(dir_str);
        //pdf_file_pathのディレクトリが存在しない場合は作成する。作成に失敗した場合はpanicする。
        fs::create_dir_all(dmpty_dir_path).expect("COULD NOT MAKE DIRECTORY.");
        let pdf_files = seek_pdf_file(dmpty_dir_path).unwrap();
        assert_eq!(pdf_files.len(), 0);
        //pdf_file_pathのディレクトリが存在している場合は削除する。削除に失敗した場合はpanicする。
        fs::remove_dir_all(dmpty_dir_path).expect("COULD NOT REMOVE DIRECTORY.");
    }

    #[test]
    fn test_seek_pdf_file_with_pdf_files() {
        let temp_dir: PathBuf = PathBuf::from("test_pdf/dummy_pdf_files_dir");

        let mut pdf_file1: PathBuf = temp_dir.clone();
        pdf_file1.push("file1.pdf");
        let full_path_pdf_file1 = fs::canonicalize(pdf_file1.as_path()).unwrap();

        let mut pdf_file2: PathBuf = temp_dir.clone();
        pdf_file2.push("file2.pdf");
        let full_path_pdf_file2 = fs::canonicalize(pdf_file2.as_path()).unwrap();

        let mut pdf_file3: PathBuf = temp_dir.clone();
        pdf_file3.push("file3.pdf");
        let full_path_pdf_file3 = fs::canonicalize(pdf_file3.as_path()).unwrap();

        let pdf_files: Vec<std::path::PathBuf> = seek_pdf_file(temp_dir.as_path()).unwrap();
        assert_eq!(pdf_files.len(), 3);
        assert!(pdf_files.contains(&full_path_pdf_file1));
        assert!(pdf_files.contains(&full_path_pdf_file2));
        assert!(pdf_files.contains(&full_path_pdf_file3));
    }

    #[test]
    fn test_seek_pdf_file_with_non_pdf_files() {
        let temp_dir: PathBuf = PathBuf::from("test_pdf/dummy_notpdf_files_dir");
        let pdf_files = seek_pdf_file(temp_dir.as_path()).unwrap();
        assert_eq!(pdf_files.len(), 0);
    }
}
