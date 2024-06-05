

use std::fs;
use std::path::{Path, PathBuf};

/// `is_valid_directory`関数は、与えられたパスが有効なディレクトリであることを確認します。
///
/// # 引数
///
/// * `directory_path` - 検証するディレクトリのパス。`Path`参照として与えられます。
///
/// # 戻り値
///
/// * ディレクトリが有効で、フルパスを取得できた場合は、そのフルパスを`Ok`でラップして返します。
/// * ディレクトリが無効である場合、またはフルパスの取得に失敗した場合は、エラーを`Err`でラップして返します。
///
/// # 例
///
/// ```
/// let path = Path::new("/some/directory");
/// let result = is_valid_directory(&path);
/// assert!(result.is_ok());
/// ```
///
/// # 注意
///
/// この関数はファイルシステムにアクセスするため、ディレクトリが有効であることを確認するためには適切なパーミッションが必要です。
pub fn is_valid_directory(directory_path: &Path) -> std::io::Result<PathBuf> {
    //引数で渡されたパスをフルパスに変換する。
    let full_path = match fs::canonicalize(directory_path) {
        Ok(path) => path,
        Err(e) => return Err(e),
    };

    let path_str_option: Option<&str> = full_path.to_str();
    //パスを文字列に直せない場合はエラーを返す。
    let path_str = match path_str_option {
        Some(s) => s,
        None => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to convert path to string")),
    };

    if !full_path.is_dir() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("{} is not a directory", path_str)));
    }
    Ok(full_path)
}

#[cfg(test)]
mod tests {
    use log::info;

    use crate::conf_logger::init_logger;

    use super::*;

    #[test]
    fn test_is_valid_directory_valid_directory() {
        init_logger();
        let path = Path::new("test_pdf/empty_dir");
        let result = is_valid_directory(&path);
        assert!(result.is_ok());
        let fill_path = result.unwrap();
        info!("full path : {}",fill_path.to_str().unwrap());
    }

    #[test]
    fn test_is_valid_directory_invalid_directory() {
        let path = Path::new("test_pdf/dummy_notpdf_files_dir/file1.txt");
        let result = is_valid_directory(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_valid_directory_failed_conversion() {
        let path = Path::new("nonexistent_dir");
        let result = is_valid_directory(&path);
        assert!(result.is_err());
    }
}