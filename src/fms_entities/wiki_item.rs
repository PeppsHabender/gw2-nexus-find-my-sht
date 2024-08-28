use crate::ui_utils::Renderable;
use crate::WIKI_ICON_ID;
use nexus::imgui::{Image, Ui};
use nexus::texture::get_texture;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiItem {
    pub title: String,
    pub pageid: usize,
    snippet: String,
}

impl WikiItem {
    fn snippet(&self) -> String {
        format!("...{}...", self.snippet)
            .replace("<span class=\"searchmatch\">", "")
            .replace("</span>", "")
    }
}

impl Renderable for WikiItem {
    fn title(&self) -> String {
        self.title.clone()
    }

    fn render_self(&self, ui: &Ui, max_width: Option<f32>) {
        ui.align_text_to_frame_padding();
        ui.text(self.title.clone());
        if ui.is_item_hovered() {
            ui.tooltip(|| {
                ui.push_text_wrap_pos_with_pos(f32::max(ui.current_column_width(), 300.0));
                ui.text(self.snippet());
            })
        }

        ui.same_line();
        if let Some(max_width) = max_width {
            ui.set_cursor_pos([max_width + 35.0, ui.cursor_pos()[1]]);
        }

        Image::new(get_texture(WIKI_ICON_ID).unwrap().id(), [20.0, 20.0]).build(ui);
        if ui.is_item_hovered() {
            ui.tooltip_text("Open in browser...")
        }

        if ui.is_item_clicked() {
            let _ = open::that(format!(
                "https://wiki.guildwars2.com/index.php?curid={}",
                self.pageid
            ));
        }
    }
}
