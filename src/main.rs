use rfd::FileDialog;
use std::fs;
use std::io::{self, Write}; // "io" = input/output - lets us read keyboard input and write to screen
// Write trait allows us to use flush() on stdout
use std::path::Path; // Path - helps us work with file paths (like C:\Users\...) // From external library "rfd" - creates the file picker window

fn main() {
    println!("=== Folder Scanner ===\n");

    println!("Opening file explorer to select a folder...");

    // "let" declares a new variable called "folder"
    // FileDialog::new() creates a new file picker dialog
    // The dot (.) chains methods together
    // .set_title() sets the window title
    // .pick_folder() opens the dialog and waits for user to select a folder
    let folder = FileDialog::new()
        .set_title("Select a folder to scan")
        .pick_folder();

    // "match" is like a switch/case statement - it handles different possibilities
    // folder could be Some(path) if user selected something, or None if they cancelled
    let base_path = match folder {
        Some(path) => path, // If folder has a value, extract it and call it "path"
        None => {
            // If folder is None (user cancelled)
            println!("No folder selected. Exiting.");
            return; // Exit the program early
        }
    };

    // {} is a placeholder - it gets replaced with base_path.display()
    // .display() converts the path to a readable format for printing
    println!("Selected folder: {}\n", base_path.display());

    // Get folder name to search for
    println!("Enter the folder name to search for:");

    // "mut" means mutable (can be changed)
    // String::new() creates a new, empty string
    let mut search_name = String::new();

    // io::stdin() gets the standard input (keyboard)
    // .read_line() reads what the user types
    // &mut means "mutable reference" - lets read_line change search_name
    // .expect() handles errors - if reading fails, print this message and crash
    io::stdin()
        .read_line(&mut search_name)
        .expect("Failed to read input");

    // .trim() removes spaces and newline from the end
    // We reassign to search_name (this is called "shadowing" in Rust)
    let search_name = search_name.trim();

    // Check if the user entered nothing
    if search_name.is_empty() {
        println!("No folder name provided. Exiting.");
        return; // Exit the program
    }

    // Print what we're searching for
    // {} placeholders get replaced by the values after the comma
    println!(
        "\nSearching for folders named '{}' in {}...\n",
        search_name,
        base_path.display()
    );

    // Scan and calculate
    // u64 = unsigned 64-bit integer (whole number, can't be negative)
    // mut = mutable, we'll be adding to this number
    let mut total_size: u64 = 0;
    let mut folder_count = 0; // Type inferred as integer

    // Call our custom function scan_directory
    // & means "reference" - we're lending the variables, not giving them away
    // &mut means "mutable reference" - the function can change these variables
    match scan_directory(&base_path, search_name, &mut total_size, &mut folder_count) {
        Ok(_) => {
            // Clear the scanning line one final time before showing results
            print!("\r{}\r", " ".repeat(100));
            io::stdout().flush().unwrap_or(());

            // If scanning succeeded (Ok), the _ means we ignore the success value
            println!("=== Results ===");
            println!("Found {} folder(s) named '{}'", folder_count, search_name);
            // "as f64" converts integer to floating point (decimal number)
            // {:.2} means "show 2 decimal places"
            // 1_048_576.0 = 1024 * 1024 (number of bytes in a megabyte)
            println!(
                "Total size: {} bytes ({:.2} MB)",
                total_size,
                total_size as f64 / 1_048_576.0
            );
        }
        Err(e) => {
            // If scanning failed (Err), e contains the error
            // eprintln! prints to error output (stderr) instead of standard output
            eprintln!("Error during scanning: {}", e);
        }
    }

    println!(
        "Done scanning, total size = {}, count = {}",
        total_size, folder_count
    )
}

fn scan_directory(
    path: &Path,          // &Path = immutable reference to a Path (can read, can't modify)
    search_name: &str,    // &str = string slice (immutable reference to text)
    total_size: &mut u64, // &mut = mutable reference, we can change this value
    folder_count: &mut usize, // usize = size type (good for counting things)
) -> io::Result<()> {
    // -> means "returns". io::Result<()> = either Ok(nothing) or Err(error)
    // ! means "not". is_dir() checks if path is a directory
    // If path is NOT a directory, return early
    if !path.is_dir() {
        return Ok(()); // Ok(()) means "success with no value"
    }

    // fs::read_dir() tries to read all items in a directory
    // It returns Result - either Ok(entries) or Err(error)
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries, // If successful, store the entries
        Err(e) => {
            // If failed (maybe no permission)
            // Print warning but don't crash - just skip this directory
            eprintln!("Warning: Cannot read directory {}: {}", path.display(), e);
            return Ok(()); // Return success anyway to continue with other folders
        }
    };

    // "for" loop - iterate through each item
    // "entries" is like a list of directory entries
    for entry in entries {
        // Each entry is also a Result (might fail to read)
        let entry = match entry {
            Ok(e) => e,         // If we can read the entry, use it
            Err(_) => continue, // If error, skip to next iteration (continue the loop)
        };

        // Get the full path of this entry
        let entry_path = entry.path();

        // Check if this entry is a directory
        if entry_path.is_dir() {
            // Display current scanning path on one line
            // \r returns cursor to start of line (carriage return)
            // We truncate the path to 80 chars to prevent wrapping
            let display_path = format!("{}", path.display());
            let truncated = if display_path.len() > 80 {
                format!("...{}", &display_path[display_path.len() - 77..])
            } else {
                display_path
            };
            print!("\rScanning: {:<80}", truncated);
            io::stdout().flush().unwrap_or(());

            // Check if this folder matches the search name
            // "if let" is a shorthand for match when we only care about one case
            // .file_name() gets just the folder name (not full path)
            // Returns Option<> - either Some(name) or None
            if let Some(folder_name) = entry_path.file_name() {
                // .to_string_lossy() converts the name to a string we can compare
                // == checks if two things are equal
                if folder_name.to_string_lossy() == search_name {
                    // Clear the scanning line before printing found folder
                    print!("\r{}\r", " ".repeat(100));
                    io::stdout().flush().unwrap_or(());

                    // * is the "dereference" operator - it accesses the value behind a reference
                    // += means "add and assign" (same as: *folder_count = *folder_count + 1)
                    *folder_count += 1;

                    // Calculate size of this folder
                    let size = calculate_dir_size(&entry_path);

                    // Add to running total
                    *total_size += size;

                    // Print that we found a matching folder
                    println!(
                        "Found: {} (Size: {} bytes, {:.2} MB)",
                        entry_path.display(),
                        size,
                        size as f64 / 1_048_576.0
                    );
                } else {
                    let _ = scan_directory(&entry_path, search_name, total_size, folder_count);
                }
            }

            // Recursively scan subdirectories (function calling itself!)
            // This lets us search folders inside folders inside folders...
            // let _ = means "call this but ignore the return value"
        }
    }

    // Return Ok(()) = success with no value
    Ok(())
}

// Function to calculate the total size of a directory and everything inside it
fn calculate_dir_size(path: &Path) -> u64 {
    // Create a variable to track the size
    // mut = mutable (we'll be adding to it)
    // 0u64 = the number 0, as an unsigned 64-bit integer
    let mut size = 0u64;

    // Try to read all items in this directory
    // "if let Ok(entries) = ..." is shorthand for "if it succeeds, use the result"
    if let Ok(entries) = fs::read_dir(path) {
        // Loop through each entry
        // .flatten() removes errors - if an entry can't be read, skip it
        for entry in entries.flatten() {
            // Get the full path of this entry
            let entry_path = entry.path();

            // Check if this entry is a file
            if entry_path.is_file() {
                // Try to get the file's metadata (info about the file)
                if let Ok(metadata) = fs::metadata(&entry_path) {
                    // .len() gives us the file size in bytes
                    // += adds it to our running total
                    size += metadata.len();
                }
            } else if entry_path.is_dir() {
                // If it's a directory, recursively calculate its size
                size += calculate_dir_size(&entry_path);
            }
        }
    }

    // Return the total size
    // In Rust, if the last line has no semicolon, it's automatically returned
    size
}
