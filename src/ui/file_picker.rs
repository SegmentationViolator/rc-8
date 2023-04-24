use std::fs;
use std::io;
use std::path;

pub struct FilePicker {
    dialog: egui_file::FileDialog,
}

impl FilePicker {
    pub fn is_open(&self) -> bool {
        self.dialog.state() == egui_file::State::Open
    }

    pub fn load(path: Option<&path::PathBuf>) -> Result<Option<Vec<u8>>, String> {
        path.and_then(|path| {
            Some(fs::read(path).map_err(|error| match error.kind() {
                io::ErrorKind::NotFound => {
                    format!(
                        "file '{}' does not exists",
                        path.file_name()
                            .and_then(|file_name| file_name.to_str())
                            .unwrap()
                    )
                }
                _ => {
                    format!("{}", error)
                }
            }))
        })
        .transpose()
    }

    pub fn new() -> Self {
        Self {
            dialog: egui_file::FileDialog::open_file(None)
                .resizable(false)
                .show_new_folder(false)
                .show_rename(false),
        }
    }

    pub fn open(&mut self) {
        self.dialog.open();
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<path::PathBuf> {
        if self.dialog.show(ctx).selected() {
            return self.dialog.path();
        }

        None
    }
}
