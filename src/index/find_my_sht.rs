use crate::index::index_reader::IndexReader;
use crate::index::item_loader::{Location, PlayerItemSpecifics};
use crate::settings::settings::Settings;
use crate::{BANK_ICON_ID, INV_ICON_ID, MAT_STORE_ID, SHARED_INV_ICON_ID};
use nexus::imgui::{Image, Ui, Window};
use nexus::texture::get_texture;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub struct ItemSearch {
    pub show: bool,
    search: String,
    old_search: String,
    last_update: Instant,
    searcher: IndexReader,
}

static mut SEARCH: OnceLock<ItemSearch> = OnceLock::new();
impl ItemSearch {
    fn new() -> Self {
        Self {
            show: false,
            search: "".to_string(),
            old_search: "".to_string(),
            last_update: Instant::now(),
            searcher: IndexReader::new(),
        }
    }

    pub fn take() -> Option<Self> {
        unsafe { SEARCH.take() }
    }

    pub fn get_mut() -> &'static mut Self {
        unsafe {
            if let Some(search) = SEARCH.get_mut() {
                return search;
            }

            let _ = SEARCH.set(Self::new());
            SEARCH.get_mut().expect("?")
        }
    }

    pub fn render(&mut self, ui: &Ui) {
        if !self.show {
            return;
        }

        Window::new("Find my Sh*t")
            .opened(&mut self.show)
            .collapsible(false)
            .resizable(false)
            .always_auto_resize(true)
            .build(ui, || {
                if ui.input_text("", &mut self.search).build() {
                    self.last_update = Instant::now();
                }

                ui.same_line();
                if let Some(last_update) = Settings::get().last_update {
                    ui.text(
                        last_update
                            .format(" Last Update: %b %d. %H:%M:%S")
                            .to_string(),
                    );
                } else {
                    ui.text(" Last Update: Unknown");
                }

                if self.old_search != self.search
                    && self.last_update.elapsed() > Duration::from_millis(500)
                {
                    self.searcher.search(self.search.clone());
                    self.old_search = self.search.clone();
                }

                for item in self.searcher.last_result.clone().lock().unwrap().iter() {
                    if let Some(icon) = get_texture(item.name.clone()) {
                        Image::new(icon.id(), [20.0, 20.0]).build(ui);
                        ui.same_line();
                    }

                    ui.align_text_to_frame_padding();
                    ui.text(&item.name);

                    let mut specifics = item.locations.clone();
                    render_location(
                        specifics.borrow_mut(),
                        ui,
                        &Location::Bank,
                        BANK_ICON_ID,
                        "in bank",
                    );
                    render_location(
                        specifics.borrow_mut(),
                        ui,
                        &Location::MaterialStorage,
                        MAT_STORE_ID,
                        "in material storage",
                    );
                    render_location(
                        specifics.borrow_mut(),
                        ui,
                        &Location::SharedInventory,
                        SHARED_INV_ICON_ID,
                        "in shared inventory",
                    );

                    if !specifics.is_empty() {
                        ui.same_line();
                        Image::new(get_texture(INV_ICON_ID).unwrap().id(), [20.0, 20.0]).build(ui);

                        for (loc, specs) in specifics.iter() {
                            match loc {
                                Location::Character(char) => {
                                    if ui.is_item_hovered() {
                                        ui.tooltip_text(format!(
                                            "{} on char {}",
                                            specs.count, char
                                        ));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            });
    }
}

fn render_location(
    specifis: &mut HashMap<Location, PlayerItemSpecifics>,
    ui: &Ui,
    location: &Location,
    texture_id: &str,
    tt_suffix: &str,
) {
    if let Some(specs) = specifis.remove(location) {
        ui.same_line();
        Image::new(get_texture(texture_id).unwrap().id(), [20.0, 20.0]).build(ui);

        if ui.is_item_hovered() {
            ui.tooltip_text(format!("{} {}", specs.count, tt_suffix));
        }
    }
}
