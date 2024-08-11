use crate::index::index_reader::IndexReader;
use crate::index::item_loader::{Location, PlayerItemSpecifics};
use crate::settings::settings::Settings;
use crate::{BANK_ICON_ID, INV_ICON_ID, MAT_STORE_ID, SHARED_INV_ICON_ID};
use nexus::imgui::{Direction, Image, Ui, Window};
use nexus::texture::get_texture;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub struct ItemSearch {
    pub show: bool,
    search: String,
    page: usize,
    old_search: String,
    last_input_update: Instant,
    searcher: IndexReader,
}

static mut SEARCH: OnceLock<ItemSearch> = OnceLock::new();
impl ItemSearch {
    fn new() -> Self {
        Self {
            show: false,
            page: 0,
            search: "".to_string(),
            old_search: "".to_string(),
            last_input_update: Instant::now(),
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
                    self.last_input_update = Instant::now();
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
                    && self.last_input_update.elapsed() > Duration::from_millis(500)
                {
                    self.page = 0;
                    self.searcher.search(self.search.clone(), self.page);
                    self.old_search = self.search.clone();
                }

                let max_width = self
                    .searcher
                    .last_result
                    .clone()
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|i| ui.calc_text_size(i.name.clone())[0])
                    .reduce(f32::max);

                for item in self.searcher.last_result.clone().lock().unwrap().iter() {
                    let mut hovered = false;
                    if let Some(icon) = get_texture(item.name.clone()) {
                        Image::new(icon.id(), [20.0, 20.0]).build(ui);
                        hovered = ui.is_item_hovered();
                        ui.same_line();
                    }

                    ui.align_text_to_frame_padding();
                    if Settings::get().color_items {
                        ui.text_colored(item.rarity.color(), &item.name);
                    } else {
                        ui.text(&item.name);
                    }

                    hovered = hovered || ui.is_item_hovered();
                    if hovered && item.description.is_some() {
                        ui.tooltip(|| {
                            ui.push_text_wrap_pos_with_pos(300.0);
                            ui.tooltip_text(&item.description.clone().unwrap());
                        })
                    }

                    ui.same_line();

                    ui.set_cursor_pos([max_width.unwrap() + 30.0, ui.cursor_pos()[1]]);

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
                        Image::new(get_texture(INV_ICON_ID).unwrap().id(), [20.0, 20.0]).build(ui);
                        ui.same_line();

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

                    ui.dummy([0.0, 0.0]);
                }

                if !self.searcher.last_result.clone().lock().unwrap().is_empty() {
                    if self.page > 0 {
                        if ui.arrow_button("##Prev", Direction::Left) {
                            self.page -= 1;
                            self.searcher.search(self.search.clone(), self.page);
                        }
                    } else {
                        ui.dummy([20.0, 20.0])
                    }

                    ui.same_line();

                    let has_more = self.searcher.has_more.clone();
                    if has_more.load(Ordering::SeqCst) {
                        if ui.arrow_button("##More", Direction::Right) {
                            self.page += 1;
                            self.searcher.search(self.search.clone(), self.page);
                        }
                    } else {
                        ui.dummy([20.0, 20.0]);
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
        Image::new(get_texture(texture_id).unwrap().id(), [20.0, 20.0]).build(ui);

        if ui.is_item_hovered() {
            ui.tooltip_text(format!("{} {}", specs.count, tt_suffix));
        }
        ui.same_line();
    }
}
