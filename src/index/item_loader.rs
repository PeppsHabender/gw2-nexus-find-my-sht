use crate::entities::{Gw2Inventory, Gw2Item, Gw2PlayerItem};
use crate::fms_entities::player_item::{Location, PlayerItem};
use crate::settings::settings::Settings;
use crate::tantivy::add_documents;
use crate::utils::{auth_request, fetch_items};
use log::{debug, error, info};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Fetches all items at all locations defined in [Location]
pub fn fetch_all_items() {
    info!("Refreshing item index...");

    // Item collector
    let items = Arc::new(Mutex::new(Vec::<Vec<PlayerItem>>::new()));

    // Join handles
    let mut handles = vec![];

    let characters = auth_request::<Vec<String>>("characters");
    // Spawn a new thread per char, this is a significant performance boost compared to the calls below
    for character in characters.unwrap_or(vec![]) {
        let items = items.clone();
        handles.push(std::thread::spawn(move || {
            let found = fetch_from_character(character.clone());
            items.lock().unwrap().push(found);
        }))
    }

    for handle in handles {
        handle.join().unwrap();
    }

    info!("Fetching items from shared inventory...");
    items
        .clone()
        .lock()
        .unwrap()
        .push(fetch_from("account/inventory", Location::SharedInventory));

    info!("Fetching items from bank...");
    items
        .clone()
        .lock()
        .unwrap()
        .push(fetch_from("account/bank", Location::Bank));

    info!("Fetching items from material storage...");
    items.clone().lock().unwrap().push(fetch_materials());

    let mut items_map: HashMap<usize, PlayerItem> = HashMap::new();
    let all_items = items.lock().unwrap();

    // Store found items inside of a map for better access
    for items in all_items.iter() {
        for item in items {
            if items_map.contains_key(&item.id) {
                items_map.get_mut(&item.id).unwrap().add(&item);
            } else {
                items_map.insert(item.id, item.clone());
            }
        }
    }

    // Index everything
    match store(items_map.iter().map(|e| e.1).collect()) {
        Ok(_) => {
            // Push update
            Settings::get_mut().update_last_update();
        }
        Err(e) => {
            error!("Failed to refresh index due to:\n{}!", e)
        }
    };
}

/// Fetches all items for the given character
fn fetch_from_character(character: String) -> Vec<PlayerItem> {
    info!("Fetching items from char {}...", character);

    match auth_request::<Gw2Inventory>(format!("characters/{character}/inventory").as_str()) {
        Err(_) => Vec::new(),
        Ok(inv) => {
            let found = inv
                .bags
                .iter()
                .flat_map(|b| b.inventory.clone())
                .flatten()
                .collect::<Vec<Gw2PlayerItem>>();

            convert(Location::Character(character.clone()), found)
        }
    }
}

/// Not included in [fetch_from] because we make sure that there is more than 0 of the items
fn fetch_materials() -> Vec<PlayerItem> {
    match auth_request::<Vec<Gw2PlayerItem>>("account/materials") {
        Err(_) => Vec::new(),
        Ok(materials) => {
            // Only handle items with count greater than 0
            let count_gz = materials
                .iter()
                .map(|i| i.clone())
                .filter(|i| i.count > 0)
                .collect::<Vec<Gw2PlayerItem>>();

            convert(Location::MaterialStorage, count_gz)
        }
    }
}

/// Fetches all items from a given endpoint, converted to the given location
fn fetch_from(endpoint: &str, location: Location) -> Vec<PlayerItem> {
    match auth_request::<Vec<Option<Gw2PlayerItem>>>(endpoint) {
        Err(_) => Vec::new(),
        Ok(inv) => {
            let found = inv.iter().flatten().map(|i| i.clone()).collect();

            convert(location.clone(), found)
        }
    }
}

fn convert(location: Location, items: Vec<Gw2PlayerItem>) -> Vec<PlayerItem> {
    let item_ids: Vec<usize> = items.iter().map(|i| i.id.clone()).collect();

    // Fetch all items from gw2 api and map them for better access
    let gw2_items = fetch_items(item_ids);
    let gw2_items_map = gw2_items
        .iter()
        .map(|i| (i.id, i))
        .collect::<HashMap<usize, &Gw2Item>>();

    // Take all items where we found ids for
    items
        .iter()
        .map(|i| {
            if gw2_items_map.contains_key(&i.id.clone()) {
                Some(PlayerItem::from(
                    location.clone(),
                    &i,
                    gw2_items_map.get(&i.id).unwrap(),
                ))
            } else {
                None
            }
        })
        .flatten()
        .collect()
}

/// Indexes given items
fn store(items: Vec<&PlayerItem>) -> anyhow::Result<()> {
    debug!("Indexing items...");
    add_documents(items.iter().map(|i| i.doc()));
    info!("Indexed Items");

    Ok(())
}
