use crate::index::find_my_sht::ItemSearch;
use crate::settings::settings::Settings;
use crate::tantivy::cleanup_tantivy;
use crate::utils::sub_path;
use nexus::imgui::Ui;
use nexus::keybind::register_keybind_with_string;
use nexus::texture::load_texture_from_memory;
use nexus::{
    gui::{register_render, RenderType},
    keybind_handler, render, AddonFlags,
};
use std::sync::OnceLock;
use std::thread::JoinHandle;
use crate::index::item_loader::fetch_all_items;

mod entities;
mod index;
mod settings;
mod utils;
mod tantivy;

nexus::export!(
    name: "Find my Sh*t",
    signature: -0x13376969,
    load,
    unload,
    flags: AddonFlags::None
);

static mut THREADS: OnceLock<Vec<JoinHandle<()>>> = OnceLock::new();

static BANK_ICON_BYTES: &'static [u8] = include_bytes!("../icons/bank.png");
static MAT_STORE_ICON_BYTES: &'static [u8] = include_bytes!("../icons/mat_store.png");
static SHARED_INV_ICON_BYTES: &'static [u8] = include_bytes!("../icons/shared_inv.png");
static INV_ICON_BYTES: &'static [u8] = include_bytes!("../icons/inv.png");

const BANK_ICON_ID: &str = "BANK_ICON";
const MAT_STORE_ID: &str = "MAT_S_ICON";
const SHARED_INV_ICON_ID: &str = "SHARED_INV_ICON";
const INV_ICON_ID: &str = "INV_ICON";

fn load() {
    load_texture_from_memory(BANK_ICON_ID, BANK_ICON_BYTES, None);
    load_texture_from_memory(MAT_STORE_ID, MAT_STORE_ICON_BYTES, None);
    load_texture_from_memory(SHARED_INV_ICON_ID, SHARED_INV_ICON_BYTES, None);
    load_texture_from_memory(INV_ICON_ID, INV_ICON_BYTES, None);

    unsafe {
        let _ = THREADS.set(vec![]);

        if let Some(settings) = Settings::from_path(sub_path("settings.json")) {
            *Settings::get_mut() = settings;
        }

        THREADS.get_mut().unwrap().push(std::thread::spawn(fetch_all_items));
    }

    register_render(RenderType::OptionsRender, render!(render_options)).revert_on_unload();
    register_render(RenderType::Render, render!(render_search)).revert_on_unload();

    let handler = keybind_handler!(|_, release| {
        if !release {
            return;
        }

        let search = ItemSearch::get_mut();
        search.show = !search.show;
    });
    register_keybind_with_string("KB_OPEN_SEARCH", handler, "ALT+S").revert_on_unload();
}

fn render_options(ui: &Ui) {
    Settings::get_mut().render(ui);
}

fn render_search(ui: &Ui) {
    ItemSearch::get_mut().render(ui);
}

fn unload() {
    unsafe {
        for t in THREADS.take().unwrap() {
            let _ = t.join();
        }

        if let Some(settings) = Settings::take() {
            let _ = settings.store(sub_path("settings.json"));
        }

        cleanup_tantivy();

        let _ = ItemSearch::take();
    }
}
