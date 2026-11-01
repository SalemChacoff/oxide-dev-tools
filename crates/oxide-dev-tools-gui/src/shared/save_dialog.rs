//! `save_dialog` — the platform save-dialog seam behind the tool-page
//! `Export…` command. Pages ask the OS for a destination path through a
//! trait, so tests can replace the real dialog with a fixed path while the
//! production implementation opens the native panel (Explorer on Windows,
//! `NSSavePanel` on macOS, XDG Desktop Portal on Linux) via `rfd`.
//!
//! Threading: `pick_save_path` is called on the GPUI main thread (rfd
//! recommends creating macOS dialogs there) and returns a boxed future the
//! caller awaits on the background executor. Cancelling the dialog resolves
//! to `None`.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use gpui_kit::SharedString;

/// Picks a destination path for an export through the platform's native
/// save dialog.
pub trait SaveFileDialog: 'static {
    /// Opens the native save dialog with `suggested_name` prefilled and
    /// resolves to the picked path; `None` means the user cancelled.
    fn pick_save_path(&self, suggested_name: SharedString) -> Pin<Box<dyn Future<Output = Option<PathBuf>> + Send>>;
}

/// The production picker: `rfd`'s async dialog over the OS-native panels.
#[derive(Clone, Copy, Debug)]
pub struct NativeSaveFileDialog;

impl SaveFileDialog for NativeSaveFileDialog {
    fn pick_save_path(&self, suggested_name: SharedString) -> Pin<Box<dyn Future<Output = Option<PathBuf>> + Send>> {
        // Built on the caller's thread (the click handler) per rfd's macOS
        // guidance; only the future crosses to the executor.
        let dialog = rfd::AsyncFileDialog::new()
            .set_file_name(suggested_name.as_ref())
            .save_file();
        Box::pin(async move { dialog.await.map(|file| file.path().to_path_buf()) })
    }
}
