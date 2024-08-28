use crate::fms_entities::player_item::{Location, PlayerItemSpecifics};
use crate::{COPPER_ICON_ID, GOLD_ICON_ID, SILVER_ICON_ID};
use nexus::imgui::{Image, StyleVar, Ui};
use nexus::texture::get_texture;
use std::collections::HashMap;

pub trait Renderable {
    fn title(&self) -> String;

    fn render_self(&self, ui: &Ui, max_width: Option<f32>);
}

pub fn render_location(
    specifics: &mut HashMap<Location, PlayerItemSpecifics>,
    ui: &Ui,
    location: &Location,
    texture_id: &str,
    tt_suffix: &str,
) {
    if let Some(specs) = specifics.remove(location) {
        Image::new(get_texture(texture_id).unwrap().id(), [20.0, 20.0]).build(ui);

        if ui.is_item_hovered() {
            ui.tooltip_text(format!("{} {}", specs.count, tt_suffix));
        }
        ui.same_line();
    }
}

pub fn build_tp(ui: &Ui, text: &str, units: (usize, usize, usize)) {
    ui.align_text_to_frame_padding();
    ui.tooltip_text(text);
    let spacing = ui.push_style_var(StyleVar::ItemSpacing([2.0, 20.0]));
    if units.0 > 0 {
        draw_gold_unit(ui, GOLD_ICON_ID, units.0)
    }
    if units.1 > 0 {
        draw_gold_unit(ui, SILVER_ICON_ID, units.1);
    }
    draw_gold_unit(ui, COPPER_ICON_ID, units.2);
    spacing.pop();
}

fn draw_gold_unit(ui: &Ui, icon: &str, unit: usize) {
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

pub fn render_description(ui: &Ui, input: &str) {
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
