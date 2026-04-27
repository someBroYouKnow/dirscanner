use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub struct ScanOutput {
    pub total_size: u64,
    pub found_folders: Vec<PathBuf>,
}

pub enum ScanProgress {
    Scanning(PathBuf),
    Found {
        path: PathBuf,
        size: u64,
    },
    Warning(String),
}

/// Same semantics as the original single-threaded demo: matching folders are sized
/// and not descended into for further name matches.
pub fn scan_directory(
    path: &Path,
    search_name: &str,
    mut on_progress: impl FnMut(ScanProgress),
) -> io::Result<ScanOutput> {
    let mut total_size = 0u64;
    let mut found_folders = Vec::new();
    scan_directory_inner(
        path,
        search_name,
        &mut total_size,
        &mut found_folders,
        &mut on_progress,
    )?;
    Ok(ScanOutput {
        total_size,
        found_folders,
    })
}

fn scan_directory_inner(
    path: &Path,
    search_name: &str,
    total_size: &mut u64,
    found_folders: &mut Vec<PathBuf>,
    on_progress: &mut impl FnMut(ScanProgress),
) -> io::Result<()> {
    if !path.is_dir() {
        return Ok(());
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => {
            if e.kind() == io::ErrorKind::PermissionDenied {
                return Ok(());
            }
            on_progress(ScanProgress::Warning(format!(
                "Cannot read directory {}: {}",
                path.display(),
                e
            )));
            return Ok(());
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let entry_path = entry.path();

        if entry_path.is_dir() {
            on_progress(ScanProgress::Scanning(path.to_path_buf()));

            if let Some(folder_name) = entry_path.file_name() {
                if folder_name.to_string_lossy() == search_name {
                    let size = calculate_dir_size(&entry_path);
                    *total_size += size;
                    found_folders.push(entry_path.clone());
                    on_progress(ScanProgress::Found {
                        path: entry_path,
                        size,
                    });
                } else {
                    scan_directory_inner(
                        &entry_path,
                        search_name,
                        total_size,
                        found_folders,
                        on_progress,
                    )?;
                }
            }
        }
    }

    Ok(())
}

pub fn calculate_dir_size(path: &Path) -> u64 {
    let mut size = 0u64;
    let mut dirs_to_process = vec![path.to_path_buf()];

    while let Some(current_dir) = dirs_to_process.pop() {
        if let Ok(entries) = fs::read_dir(&current_dir) {
            for entry in entries.flatten() {
                let entry_path = entry.path();

                if entry_path.is_file() {
                    if let Ok(metadata) = fs::metadata(&entry_path) {
                        size += metadata.len();
                    }
                } else if entry_path.is_dir() {
                    dirs_to_process.push(entry_path);
                }
            }
        }
    }

    size
}

pub struct DeleteSummary {
    pub deleted: usize,
    pub failed: usize,
}

pub fn delete_folders(folders: &[PathBuf]) -> DeleteSummary {
    let mut deleted = 0usize;
    let mut failed = 0usize;

    for folder in folders {
        match fs::remove_dir_all(folder) {
            Ok(_) => deleted += 1,
            Err(_) => failed += 1,
        }
    }

    DeleteSummary { deleted, failed }
}
