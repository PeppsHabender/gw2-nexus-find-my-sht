use chrono::{DateTime, Local};
use nexus::imgui::Ui;
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::fs::{create_dir_all, File};
use std::path::Path;
use std::sync::OnceLock;

use crate::entities::LoadingState;
use crate::settings::api_key_loader::ApiKeyLoader;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Settings {
    pub api_key: String,
    pub last_update: Option<DateTime<Local>>,
    #[serde(skip)]
    temp_api_key: String,
    #[serde(skip)]
    loader: ApiKeyLoader,
}

static mut SETTINGS: OnceLock<Settings> = OnceLock::new();
impl Settings {
    pub fn new() -> Self {
        Self {
            api_key: "".to_string(),
            temp_api_key: "".to_string(),
            loader: ApiKeyLoader::new(),
            last_update: None
        }
    }

    pub fn update_last_update(&mut self) {
        self.last_update = Some(Local::now());
    }

    pub fn take() -> Option<Self> {
        unsafe { SETTINGS.take() }
    }

    pub fn get_mut() -> &'static mut Self {
        unsafe {
            if let Some(settings) = SETTINGS.get_mut() {
                return settings;
            }

            let _ = SETTINGS.set(Self::new());
            Self::get_mut()
        }
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Option<Self> {
        if let Ok(str) = std::fs::read_to_string(path) {
            if let Ok(mut settings) = serde_json::from_str::<Self>(&str) {
                settings.temp_api_key = settings.api_key.clone();
                return Some(settings);
            }
        }

        None
    }

    pub fn store<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let path = path.as_ref();

        create_dir_all(path.parent().unwrap())?;
        let mut file = File::options()
            .write(true)
            .append(false)
            .create(true)
            .truncate(true)
            .open(path)?;

        Ok(serde_json::to_writer_pretty(&mut file, self)?)
    }

    pub fn get() -> &'static Self {
        unsafe { SETTINGS.get_or_init(|| Self::new()) }
    }

    pub fn render(&mut self, ui: &Ui) {
        thread_local! {
            static EDIT: Cell<bool> = Cell::new(false);
            static API_KEY: RefCell<String> = RefCell::new(String::new());
        }

        self.loader.update();
        let mut edit = EDIT.get();

        ui.input_text("Api Key", &mut self.temp_api_key)
            .read_only(!edit)
            .password(!edit)
            .build();
        if ui.is_item_hovered() {
            ui.tooltip_text("Please provide an API Key with the following permissions:\nAccount,Inventories");
        }

        match self.loader.loading_state() {
            LoadingState::Loading => {
                self.loader.clone().update();
                ui.text(format!("Verifying{}", self.loader.curr_dots()));
            }
            _ => {
                ui.same_line();
                if edit {
                    edit = !ui.button("Set");
                    if !edit {
                        self.loader
                            .clone()
                            .verify_api_key(self.temp_api_key.clone());
                    }
                } else {
                    edit = ui.button("Edit");
                }
            }
        }

        match self.loader.loading_state() {
            LoadingState::Error(msg) => ui.text_colored([213., 0., 0., 1.], msg),
            LoadingState::Success(api_key) => {
                self.api_key = api_key;
                ui.text_colored([0.0, 0.5, 0.0, 1.0], "Valid Key");
            }
            _ => {}
        }

        EDIT.set(edit);
    }
}
