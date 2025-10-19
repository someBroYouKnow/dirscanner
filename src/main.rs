use std::fs;
use std::io;
use std::path::Path;
use rfd::FileDialog;

fn main() {
    println!("=== Folder Scanner ===\n");

    // Open file dialog to select folder
    println!("Opening file explorer to select a folder...");
    let folder = FileDialog::new()
        .set_title("Select a folder to scan")
        .pick_folder();

    let base_path = match folder {
        Some(path) => path,
        None => {
            println!("No folder selected. Exiting.");
            return;
        }
    };

    println!("Selected folder: {}\n", base_path.display());

    // Get folder name to search for
    println!("Enter the folder name to search for:");
    let mut search_name = String::new();
    io::stdin()
        .read_line(&mut search_name)
        .expect("Failed to read input");
    let search_name = search_name.trim();

    if search_name.is_empty() {
        println!("No folder name provided. Exiting.");
        return;
    }

    println!("\nSearching for folders named '{}' in {}...\n", search_name, base_path.display());

    // Scan and calculate
    let mut total_size: u64 = 0;
    let mut folder_count = 0;

    match scan_directory(&base_path, search_name, &mut total_size, &mut folder_count) {
        Ok(_) => {
            println!("\n=== Results ===");
            println!("Found {} folder(s) named '{}'", folder_count, search_name);
            println!("Total size: {} bytes ({:.2} MB)", total_size, total_size as f64 / 1_048_576.0);
        }
        Err(e) => {
            eprintln!("Error during scanning: {}", e);
        }
    }
}

fn scan_directory(
    path: &Path,
    search_name: &str,
    total_size: &mut u64,
    folder_count: &mut usize,
) -> io::Result<()> {
    if !path.is_dir() {
        return Ok(());
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Warning: Cannot read directory {}: {}", path.display(), e);
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
            // Check if this folder matches the search name
            if let Some(folder_name) = entry_path.file_name() {
                if folder_name.to_string_lossy() == search_name {
                    *folder_count += 1;
                    let size = calculate_dir_size(&entry_path);
                    *total_size += size;
                    println!("Found: {} (Size: {} bytes, {:.2} MB)",
                             entry_path.display(),
                             size,
                             size as f64 / 1_048_576.0);
                }
            }

            // Recursively scan subdirectories
            let _ = scan_directory(&entry_path, search_name, total_size, folder_count);
        }
    }

    Ok(())
}

fn calculate_dir_size(path: &Path) -> u64 {
    let mut size = 0u64;

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();

            if entry_path.is_file() {
                if let Ok(metadata) = fs::metadata(&entry_path) {
                    size += metadata.len();
                }
            } else if entry_path.is_dir() {
                size += calculate_dir_size(&entry_path);
            }
        }
    }

    size
}
