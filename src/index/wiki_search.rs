use crate::fms_entities::wiki_item::WikiItem;
use crate::settings::settings::Settings;
use crate::spawn_thread;
use crate::utils::Searcher;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WikiContinue {
    sroffset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WikiSearch {
    search: Vec<WikiItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WikiResult {
    #[serde(alias = "continue")]
    more: Option<WikiContinue>,
    query: WikiSearch,
}

pub struct WikiReader {
    last_result: Arc<Mutex<Vec<WikiItem>>>,
    has_more: Arc<AtomicBool>,
    loading: Arc<Mutex<bool>>,
}

impl WikiReader {
    pub fn new() -> Self {
        Self {
            last_result: Arc::new(Mutex::new(vec![])),
            has_more: Arc::new(AtomicBool::new(false)),
            loading: Arc::new(Mutex::new(false)),
        }
    }

    fn search_for(
        text: String,
        page: usize,
        has_more: Arc<AtomicBool>,
    ) -> anyhow::Result<Vec<WikiItem>> {
        let limit = Settings::get().item_load_limit;
        let text = text.replace(" ", "%20");
        let url = format!("https://wiki.guildwars2.com/api.php?action=query&list=search&srsearch={text}&utf8=&format=json&srlimit={limit}&sroffset={page}");

        let found = ureq::get(&url).call()?.into_json::<WikiResult>()?;
        if found.more.is_some() {
            has_more.clone().store(true, Ordering::SeqCst);
        }

        Ok(found.query.search)
    }
}

impl Searcher<Vec<WikiItem>> for WikiReader {
    fn is_loading(&self) -> bool {
        self.loading.clone().lock().unwrap().clone()
    }

    fn has_more(&self) -> bool {
        self.has_more.load(Ordering::SeqCst)
    }

    fn search(&self, text: String, page: usize) {
        let last_result = self.last_result.clone();
        let loading = self.loading.clone();
        let has_more = self.has_more.clone();

        spawn_thread(move || {
            // Uuuuh no idea why I'm doing this, but never change a running system
            while *loading.lock().unwrap() {
                std::thread::sleep(Duration::from_millis(20));
            }

            *loading.lock().unwrap() = true;
            let result = WikiReader::search_for(text.clone().to_lowercase(), page, has_more);
            *loading.lock().unwrap() = false;

            *last_result.lock().unwrap() = result.unwrap_or_else(|_| vec![])
        })
    }

    fn last_result(&self) -> Vec<WikiItem> {
        (*self.last_result.clone().lock().unwrap()).to_owned()
    }
}
