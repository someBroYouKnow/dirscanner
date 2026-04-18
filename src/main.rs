mod learn_threads;
mod scanner;

use scanner::{delete_folders, scan_directory, ScanProgress};
use slint::invoke_from_event_loop;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;

    let found_store: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(Vec::new()));

    let ui_weak_browse = ui.as_weak();
    ui.on_browse_clicked(move || {
        let Some(ui) = ui_weak_browse.upgrade() else {
            return;
        };
        let folder = rfd::FileDialog::new()
            .set_title("Select a folder to scan")
            .pick_folder();
        if let Some(path) = folder {
            ui.set_root_path(path.display().to_string().into());
        }
    });

    let found_for_scan = Arc::clone(&found_store);
    let ui_weak_scan = ui.as_weak();
    ui.on_scan_clicked(move || {
        let Some(ui) = ui_weak_scan.upgrade() else {
            return;
        };

        let root = PathBuf::from(ui.get_root_path().to_string());
        let search = ui.get_search_query().to_string();
        if search.trim().is_empty() || !root.is_dir() {
            ui.set_status_text("Select a valid root folder and search name.".into());
            return;
        }

        ui.set_busy(true);
        ui.set_status_text("Scanning…".into());
        ui.set_log_text("".into());
        ui.set_summary_text("".into());
        ui.set_found_count(0);
        if let Ok(mut g) = found_for_scan.lock() {
            g.clear();
        }

        let found_for_scan = Arc::clone(&found_for_scan);
        let ui_weak = ui.as_weak();
        let search = search.trim().to_string();
        thread::spawn(move || {
            let result = scan_directory(&root, &search, |ev| {
                let w = ui_weak.clone();
                let _ = invoke_from_event_loop(move || {
                    let Some(ui) = w.upgrade() else {
                        return;
                    };
                    match ev {
                        ScanProgress::Scanning(p) => {
                            ui.set_status_text(
                                format!("Scanning: {}", truncate_status(&p)).into(),
                            );
                        }
                        ScanProgress::Found { path, size } => {
                            let line = format!(
                                "Found: {} — {} bytes ({:.2} MB)\n",
                                path.display(),
                                size,
                                size as f64 / 1_048_576.0
                            );
                            let mut next = ui.get_log_text().to_string();
                            next.push_str(&line);
                            ui.set_log_text(next.into());
                        }
                        ScanProgress::Warning(msg) => {
                            let mut next = ui.get_log_text().to_string();
                            next.push_str(&format!("Warning: {msg}\n"));
                            ui.set_log_text(next.into());
                        }
                    }
                });
            });

            let _ = invoke_from_event_loop(move || {
                let Some(ui) = ui_weak.upgrade() else {
                    return;
                };
                ui.set_busy(false);
                match result {
                    Ok(out) => {
                        if let Ok(mut g) = found_for_scan.lock() {
                            *g = out.found_folders.clone();
                        }
                        ui.set_found_count(out.found_folders.len() as i32);
                        ui.set_status_text("Done.".into());
                        ui.set_summary_text(
                            format!(
                                "Found {} folder(s) named '{}'. Total size: {} bytes ({:.2} MB)",
                                out.found_folders.len(),
                                search,
                                out.total_size,
                                out.total_size as f64 / 1_048_576.0
                            )
                            .into(),
                        );
                    }
                    Err(e) => {
                        ui.set_status_text(format!("Error: {e}").into());
                    }
                }
            });
        });
    });

    let found_for_delete = Arc::clone(&found_store);
    let ui_weak_del = ui.as_weak();
    ui.on_delete_clicked(move || {
        let Some(ui) = ui_weak_del.upgrade() else {
            return;
        };
        let paths = found_for_delete
            .lock()
            .ok()
            .map(|g| (*g).clone())
            .unwrap_or_default();
        if paths.is_empty() {
            return;
        }

        ui.set_busy(true);
        ui.set_status_text("Deleting…".into());

        let found_for_delete = Arc::clone(&found_for_delete);
        let ui_weak = ui.as_weak();
        thread::spawn(move || {
            let summary = delete_folders(&paths);
            let _ = invoke_from_event_loop(move || {
                let Some(ui) = ui_weak.upgrade() else {
                    return;
                };
                ui.set_busy(false);
                let mut next = ui.get_log_text().to_string();
                next.push_str(&format!(
                    "\n— Deletion finished: {} deleted, {} failed.\n",
                    summary.deleted, summary.failed
                ));
                ui.set_log_text(next.into());
                ui.set_status_text("Deletion finished.".into());
                ui.set_found_count(0);
                ui.set_summary_text("".into());
                if let Ok(mut g) = found_for_delete.lock() {
                    g.clear();
                }
            });
        });
    });

    ui.run()
}

fn truncate_status(p: &std::path::Path) -> String {
    let s = p.display().to_string();
    if s.len() <= 96 {
        s
    } else {
        format!("...{}", &s[s.len() - 93..])
    }
}
