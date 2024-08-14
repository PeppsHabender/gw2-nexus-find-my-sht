use log::{error, info};
use nexus::texture::{get_texture, load_texture_from_url};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::Value;
use tantivy::TantivyDocument;

use crate::entities::Gw2Tp;
use crate::index::item_loader::PlayerItem;
use crate::settings::settings::Settings;
use crate::spawn_thread;
use crate::tantivy::{index_searcher, tantivy_index, TantivySchema};
use crate::utils::auth_request;

/// Contains all tantivy results from the last search
pub struct IndexReader {
    pub last_result: Arc<Mutex<Vec<PlayerItem>>>,
    pub has_more: Arc<AtomicBool>,
    loading: Arc<Mutex<bool>>,
}

impl IndexReader {
    pub fn new() -> Self {
        Self {
            last_result: Arc::new(Mutex::new(vec![])),
            has_more: Arc::new(AtomicBool::new(false)),
            loading: Arc::new(Mutex::new(false)),
        }
    }

    pub fn search(&mut self, text: String, page: usize) {
        let last_result = self.last_result.clone();
        let loading = self.loading.clone();
        let has_more = self.has_more.clone();

        spawn_thread(move || {
            // Uuuuh no idea why I'm doing this, but never change a running system
            while *loading.lock().unwrap() {
                std::thread::sleep(Duration::from_millis(20));
            }

            *loading.lock().unwrap() = true;
            let result = Self::search_for(text.clone().to_lowercase(), page, has_more);
            *loading.lock().unwrap() = false;
            match result {
                Ok(res) => {
                    *last_result.lock().unwrap() = res.clone();

                    let ids = res
                        .iter()
                        .map(|i| i.id.clone().to_string())
                        .collect::<Vec<_>>()
                        .join(",");

                    let last_result = last_result.clone();
                    spawn_thread(move || {
                        match auth_request::<Vec<Gw2Tp>>(&format!("commerce/prices?ids={ids}")) {
                            Ok(tp) => {
                                let tp = tp
                                    .iter()
                                    .map(|tp| (tp.id, tp))
                                    .collect::<HashMap<usize, &Gw2Tp>>();
                                if tp.is_empty() {
                                    return;
                                }

                                let len = last_result.lock().unwrap().len();
                                for i in 0..len {
                                    let id = last_result.lock().unwrap()[i].id;
                                    let tp = tp.get(&id).map(|x| **x);
                                    last_result.lock().unwrap()[i].set_tp(tp);
                                }
                            }
                            Err(e) => {
                                error!("{e}");
                            }
                        }
                    });
                }
                Err(_) => {}
            }
        });
    }

    fn search_for(
        text: String,
        page: usize,
        has_more: Arc<AtomicBool>,
    ) -> anyhow::Result<Vec<PlayerItem>> {
        let index = tantivy_index();
        let searcher = index_searcher();

        let schema: TantivySchema = index.schema().into();
        let mut parser = QueryParser::for_index(index, vec![schema.name_field, schema.descr_field]);
        // Put some more importance on names
        parser.set_field_boost(schema.name_field, 2.0);
        parser.set_conjunction_by_default();

        let query = parser.parse_query(&text)?;
        let limit = Settings::get().item_load_limit as usize;
        let top_docs = searcher.search(
            &query,
            &TopDocs::with_limit(limit + 1).and_offset(limit * page),
        )?;

        let mut found: Vec<PlayerItem> = vec![];
        for (_, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            if let Some(item) = retrieved_doc.get_first(schema.item_field) {
                found.push(rmp_serde::from_slice(item.as_bytes().unwrap()).unwrap())
            }
        }

        has_more.store(found.len() > limit, Ordering::SeqCst);
        found.truncate(limit);

        // Load all icons for the search results
        for item in &found {
            if let Some(icon) = item.icon.clone() {
                if get_texture(item.name.clone()).is_none() {
                    load_texture_from_url(
                        item.name.clone(),
                        "https://render.guildwars2.com",
                        icon.strip_prefix("https://render.guildwars2.com").unwrap(),
                        None,
                    );
                }
            }
        }

        Ok(found)
    }
}
