use std::sync::{Arc, Mutex};
use std::time::Duration;

use nexus::texture::{get_texture, load_texture_from_url};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::Value;
use tantivy::TantivyDocument;

use crate::index::item_loader::PlayerItem;
use crate::tantivy::{index_searcher, tantivy_index, TantivySchema};
use crate::THREADS;

/// Contains all tantivy results from the last search
pub struct IndexReader {
    pub last_result: Arc<Mutex<Vec<PlayerItem>>>,
    loading: Arc<Mutex<bool>>,
}

impl IndexReader {
    pub fn new() -> Self {
        Self {
            last_result: Arc::new(Mutex::new(vec![])),
            loading: Arc::new(Mutex::new(false)),
        }
    }

    pub fn search(&mut self, text: String) {
        let last_result = self.last_result.clone();
        let loading = self.loading.clone();

        unsafe {
            THREADS.get_mut().unwrap().push(std::thread::spawn(move || {
                // Uuuuh not idea what I'm doing here, but never change a running system
                while *loading.lock().unwrap() {
                    std::thread::sleep(Duration::from_millis(20));
                }

                *loading.lock().unwrap() = true;
                let result = Self::search_for(text.clone().to_lowercase());
                *loading.lock().unwrap() = false;
                match result {
                    Ok(res) => {
                        *last_result.lock().unwrap() = res;
                    }
                    Err(_) => {}
                }
            }));
        }
    }

    fn search_for(text: String) -> anyhow::Result<Vec<PlayerItem>> {
        let index = tantivy_index();
        let searcher = index_searcher();

        let schema: TantivySchema = index.schema().into();
        let mut parser =
            QueryParser::for_index(index, vec![schema.name_field, schema.descr_field]);
        // Put some more importance on names
        parser.set_field_boost(schema.name_field, 2.0);
        parser.set_conjunction_by_default();

        let query = parser.parse_query(&text)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;

        let mut found: Vec<PlayerItem> = vec![];
        for (_, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            if let Some(item) = retrieved_doc.get_first(schema.item_field) {
                found.push(rmp_serde::from_slice(item.as_bytes().unwrap()).unwrap())
            }
        }

        // Load all icons for the search results
        for item in &found {
            if let Some(icon) = item.icon.clone() {
                if get_texture(item.name.clone()).is_none() {
                    load_texture_from_url(
                        item.name.clone(),
                        "https://render.guildwars2.com",
                        icon.strip_prefix("https://render.guildwars2.com").unwrap(),
                        None
                    );
                }
            }
        }

        Ok(found)
    }
}
