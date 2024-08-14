use crate::index::index_reader::IndexReader;
use crate::index::item_loader::{Location, PlayerItem, PlayerItemSpecifics};
use crate::settings::settings::Settings;
use crate::{
    BANK_ICON_ID, COPPER_ICON_ID, GOLD_ICON_ID, INV_ICON_ID, MAT_STORE_ID, SHARED_INV_ICON_ID,
    SILVER_ICON_BYTES, SILVER_ICON_ID,
};
use nexus::imgui::{Direction, Image, StyleVar, Ui, Window};
use nexus::texture::get_texture;
use std::borrow::BorrowMut;
use std::cmp::max;
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
                    if hovered && (item.description.is_some() || item.tp_info.is_some()) {
                        ui.tooltip(|| {
                            if let Some(tp_info) = item.tp_info {
                                build_tp(ui, "Buys:", tp_info.buys.units());
                                ui.same_line();
                                ui.text("|");
                                ui.same_line();
                                build_tp(ui, "Sells:", tp_info.sells.units());
                            }

                            ui.push_text_wrap_pos_with_pos(f32::max(
                                ui.current_column_width(),
                                300.0,
                            ));

                            if let Some(description) = &item.description.clone() {
                                render_colored_text(ui, description);
                            }
                        })
                    }

                    ui.same_line();

                    ui.set_cursor_pos([max_width.unwrap() + 35.0, ui.cursor_pos()[1]]);

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

fn build_tp(ui: &Ui, text: &str, units: (usize, usize, usize)) {
    ui.align_text_to_frame_padding();
    ui.tooltip_text(text);
    let spacing = ui.push_style_var(StyleVar::ItemSpacing([2.0, 20.0]));
    if units.0 > 0 {
        draw_unit(ui, GOLD_ICON_ID, units.0)
    }
    if units.1 > 0 {
        draw_unit(ui, SILVER_ICON_ID, units.1);
    }
    draw_unit(ui, COPPER_ICON_ID, units.2);
    spacing.pop();
}

fn draw_unit(ui: &Ui, icon: &str, unit: usize) {
    ui.same_line();
    ui.align_text_to_frame_padding();
    ui.tooltip_text(format!("{}", unit));
    ui.same_line();
    ui.set_cursor_pos([ui.cursor_pos()[0], ui.cursor_pos()[1] + 5.0]);
    Image::new(get_texture(icon).unwrap().id(), [10.0, 10.0]).build(ui);
    ui.same_line();
    ui.set_cursor_pos([ui.cursor_pos()[0], ui.cursor_pos()[1] - 5.0]);
    ui.dummy([0.0, 20.0]);
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

fn render_colored_text(ui: &Ui, input: &str) {
    let mut pos = 0;
    while pos < input.len() {
        if let Some(start_tag) = input[pos..].find("<c=@") {
            // Render plain text before the tag
            if start_tag > 0 {
                ui.text(&input[pos..pos + start_tag]);
            }
            pos += start_tag;

            // Find end of the tag
            if let Some(end_tag) = input[pos..].find('>') {
                let tag = &input[pos + 4..pos + end_tag]; // Extract the tag name
                pos += end_tag + 1;

                // Find the closing </c> tag
                if let Some(close_tag) = input[pos..].find("</c>") {
                    let colored_text = &input[pos..pos + close_tag];
                    pos += close_tag + 4;

                    // Set color based on the tag
                    let color = match tag {
                        "flavor" => [0.5686, 0.8157, 0.8196, 1.0],
                        "reminder" => [0.3176, 0.5725, 0.9451, 1.0],
                        "warning" => [1.0, 0.0, 0.0, 1.0],
                        "abilitytype" => [1.0, 1.0, 1.0, 1.0],
                        _ => [1.0, 1.0, 1.0, 1.0], // Default to white
                    };

                    // Render the colored text
                    ui.text_colored(color, colored_text);
                }
            }
        } else {
            // Render any remaining plain text
            ui.text(&input[pos..]);
            break;
        }
    }
}
