use std::{fs, path::{Path, PathBuf}};

/// `seek_pdf_file`関数は、指定されたディレクトリ内のすべてのPDFファイルを探し、そのパスを返します。
///
/// # 引数
///
/// * `directory_path` - PDFファイルを探すディレクトリのパス。`Path`参照として与えられます。
///
/// # 戻り値
///
/// * 成功した場合は、PDFファイルのパスを含む`Vec<PathBuf>`を`Ok`でラップして返します。
/// * ディレクトリの読み取りに失敗した場合は、エラーを`Err`でラップして返します。
///
/// # 例
///
/// ```
/// let path = Path::new("/some/directory");
/// let result = seek_pdf_file(&path);
/// assert!(result.is_ok());
/// ```
///
/// # 注意
///
/// この関数はファイルシステムにアクセスするため、ディレクトリの読み取りには適切なパーミッションが必要です。

pub fn seek_pdf_file(directory_path: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut pdf_files = Vec::new();

    if directory_path.is_dir() {
        for entry in fs::read_dir(directory_path)? {
            let entry: fs::DirEntry = entry?;
            let path: PathBuf = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("pdf") {
                pdf_files.push(path);
            }
        }
    }

    Ok(pdf_files)
}

#[cfg(test)]
mod tests {
    use super::*;
#[test]
fn test_seek_pdf_file_empty_directory() {
    let empty_dir = Path::new("test_pdf/empty_dir");
    let pdf_files = seek_pdf_file(empty_dir).unwrap();
    assert_eq!(pdf_files.len(), 0);
}

#[test]
fn test_seek_pdf_file_with_pdf_files() {
    let temp_dir: PathBuf = PathBuf::from("test_pdf/dummy_pdf_files_dir");


    let mut pdf_file1: PathBuf = temp_dir.clone();
    pdf_file1.push("file1.pdf");
    let mut pdf_file2: PathBuf = temp_dir.clone();
    pdf_file2.push("file2.pdf");
    let mut pdf_file3: PathBuf = temp_dir.clone();
    pdf_file3.push("file3.pdf");

    let pdf_files: Vec<std::path::PathBuf> = seek_pdf_file(temp_dir.as_path()).unwrap();
    assert_eq!(pdf_files.len(), 3);
    assert!(pdf_files.contains(&pdf_file1));
    assert!(pdf_files.contains(&pdf_file2));
    assert!(pdf_files.contains(&pdf_file3));
}

#[test]
fn test_seek_pdf_file_with_non_pdf_files() {
    let temp_dir: PathBuf = PathBuf::from("test_pdf/dummy_notpdf_files_dir");
    let pdf_files = seek_pdf_file(temp_dir.as_path()).unwrap();
    assert_eq!(pdf_files.len(), 0);
}

}