use crate::entities::{Gw2Item, Gw2ItemType, Gw2PlayerItem, Gw2Rarity, Gw2Tp};
use crate::settings::settings::Settings;
use crate::tantivy::{tantivy_index, TantivySchema};
use crate::ui_utils::{build_tp, render_description, render_location, Renderable};
use crate::{
    spawn_thread, BANK_ICON_ID, INV_ICON_ID, MAT_STORE_ID, SHARED_INV_ICON_ID, WIKI_ICON_ID,
};
use nexus::imgui::{Image, Ui};
use nexus::texture::get_texture;
use serde::{Deserialize, Serialize};
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};
use tantivy::{doc, TantivyDocument};

/// Defines where a specific item lies on the account
/// TODO: Legendary Armory, Equipments
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub enum Location {
    Character(String),
    Bank,
    SharedInventory,
    MaterialStorage,
}

/// Contains specific information for an item at a certain location
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerItemSpecifics {
    pub count: usize,
    pub charges: usize,
    pub upgrades: Vec<usize>,
    pub infusions: Vec<usize>,
}

/// Find my sh*t specific player item which is stored and used for indexing
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PlayerItem {
    pub id: usize,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub item_type: Gw2ItemType,
    pub rarity: Gw2Rarity,
    pub locations: HashMap<Location, PlayerItemSpecifics>,
    #[serde(skip)]
    pub tp_info: Option<Gw2Tp>,
    #[serde(skip)]
    wikiable: Arc<RwLock<Option<bool>>>,
}

impl PlayerItem {
    pub(crate) fn doc(&self) -> TantivyDocument {
        let schema: TantivySchema = tantivy_index().schema().into();

        doc!(
            schema.id_field => self.id as u64,
            schema.name_field => self.name.clone().to_lowercase(),
            schema.descr_field => self.description.clone().unwrap_or("".to_string()).to_lowercase(),
            schema.item_field => rmp_serde::to_vec(self).expect("to be serialized")
        )
    }

    pub fn from(location: Location, item: &Gw2PlayerItem, gw2item: &Gw2Item) -> Self {
        Self {
            id: item.id,
            name: gw2item.name.clone(),
            description: gw2item.description.clone(),
            icon: gw2item.icon.clone(),
            item_type: gw2item.item_type.clone(),
            rarity: gw2item.rarity.clone(),
            locations: HashMap::from([(
                location,
                PlayerItemSpecifics {
                    count: item.count,
                    charges: item.charges.unwrap_or(0),
                    upgrades: item.upgrades.clone().unwrap_or(vec![]),
                    infusions: item.infusions.clone().unwrap_or(vec![]),
                },
            )]),
            tp_info: None,
            wikiable: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_tp(&mut self, tp: Option<Gw2Tp>) {
        self.tp_info = tp;
    }

    pub(crate) fn add(&mut self, item: &PlayerItem) {
        for (loc, spec) in &item.locations {
            if let Some(curr) = self.locations.get_mut(loc) {
                curr.count += spec.count.clone();
                curr.charges += spec.charges.clone();
                curr.infusions.append(&mut spec.infusions.clone());
                curr.upgrades.append(&mut spec.upgrades.clone());
            } else {
                self.locations.insert(loc.clone(), spec.clone());
            }
        }
    }

    fn render_wiki(&self, ui: &Ui) {
        let wikiable = *self.wikiable.read().unwrap();
        let url = format!(
            "https://wiki.guildwars2.com/wiki/{}",
            self.name.replace(" ", "_")
        );
        if wikiable.is_none() {
            *self.wikiable.write().unwrap() = Some(false);

            let wikiable = self.wikiable.clone();
            spawn_thread(move || match ureq::head(&url).call() {
                Ok(_) => *wikiable.write().unwrap() = Some(true),
                Err(_) => {}
            });
            return;
        } else if let Some(false) = wikiable {
            return;
        }

        ui.same_line();
        Image::new(get_texture(WIKI_ICON_ID).unwrap().id(), [20.0, 20.0]).build(ui);
        ui.same_line();
        if ui.is_item_hovered() {
            ui.tooltip_text("Open in wiki...")
        }

        if ui.is_item_clicked() {
            let _ = open::that(url);
        }
    }
}

impl Hash for PlayerItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.id)
    }
}

impl Renderable for PlayerItem {
    fn title(&self) -> String {
        self.name.clone()
    }

    fn render_self(&self, ui: &Ui, max_width: Option<f32>) {
        let mut hovered = false;
        if let Some(icon) = get_texture(self.name.clone()) {
            Image::new(icon.id(), [20.0, 20.0]).build(ui);
            hovered = ui.is_item_hovered();
            ui.same_line();
        }

        ui.align_text_to_frame_padding();
        if Settings::get().color_items {
            ui.text_colored(self.rarity.color(), &self.name);
        } else {
            ui.text(&self.name);
        }

        hovered = hovered || ui.is_item_hovered();
        if hovered && (self.description.is_some() || self.tp_info.is_some()) {
            ui.tooltip(|| {
                if let Some(tp_info) = self.tp_info {
                    build_tp(ui, "Buys:", tp_info.buys.units());
                    ui.same_line();
                    ui.text("|");
                    ui.same_line();
                    build_tp(ui, "Sells:", tp_info.sells.units());
                }

                ui.push_text_wrap_pos_with_pos(f32::max(ui.current_column_width(), 300.0));

                if let Some(description) = &self.description.clone() {
                    render_description(ui, description);
                }
            })
        }

        ui.same_line();

        if let Some(max_width) = max_width {
            ui.set_cursor_pos([max_width + 35.0, ui.cursor_pos()[1]]);
        }

        let mut specifics = self.locations.clone();
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
                if let Location::Character(char) = loc {
                    if ui.is_item_hovered() {
                        ui.tooltip_text(format!("{} on char {}", specs.count, char));
                    }
                }
            }
        }

        self.render_wiki(ui);
        ui.new_line();
    }
}
