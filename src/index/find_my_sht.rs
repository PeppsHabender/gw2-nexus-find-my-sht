use crate::index::index_reader::IndexReader;
use crate::index::wiki_search::WikiReader;
use crate::settings::settings::Settings;
use crate::ui_utils::Renderable;
use crate::utils::Searcher;
use nexus::imgui::{Direction, Ui, Window};
use std::borrow::{BorrowMut, Cow};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

pub struct ItemSearch {
    pub show: bool,
    search: String,
    page: Arc<AtomicUsize>,
    old_search: String,
    last_input_update: Instant,
    acc_searcher: IndexReader,
    wiki_searcher: WikiReader,
    search_type: SearchType,
}

#[derive(PartialEq, Eq)]
pub enum SearchType {
    Account,
    Wiki,
}

static mut SEARCH: OnceLock<ItemSearch> = OnceLock::new();
impl ItemSearch {
    fn new() -> Self {
        Self {
            show: false,
            page: Arc::new(AtomicUsize::new(0)),
            search: "".to_string(),
            old_search: "".to_string(),
            last_input_update: Instant::now(),
            acc_searcher: IndexReader::new(),
            wiki_searcher: WikiReader::new(),
            search_type: SearchType::Account,
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
            .focus_on_appearing(true)
            .build(ui, || {
                if ui.input_text("", &mut self.search).build() {
                    self.last_input_update = Instant::now();
                }

                if ui.is_window_appearing() {
                    ui.set_keyboard_focus_here();
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

                let items = ["Account", "Wiki"];

                let mut current_index = match self.search_type {
                    SearchType::Account => 0,
                    SearchType::Wiki => 1,
                };

                ui.combo("##Type", &mut current_index, &items, |item| {
                    Cow::from(item.to_string())
                });

                let search_type = match current_index {
                    0 => SearchType::Account,
                    1 => SearchType::Wiki,
                    _ => SearchType::Account,
                };

                let mut do_search = false;
                if search_type != self.search_type {
                    self.search_type = search_type;
                    do_search = true;
                }

                if self.old_search != self.search
                    && self.last_input_update.elapsed() > Duration::from_millis(500)
                {
                    do_search = true;
                }

                if do_search {
                    self.page.store(0, Ordering::SeqCst);
                    match self.search_type {
                        SearchType::Account => self
                            .acc_searcher
                            .search(self.search.clone(), self.page.load(Ordering::SeqCst)),
                        SearchType::Wiki => self
                            .wiki_searcher
                            .search(self.search.clone(), self.page.load(Ordering::SeqCst)),
                    }
                    self.old_search = self.search.clone();
                }

                match self.search_type {
                    SearchType::Account => {
                        let acc_searcher = self.acc_searcher.borrow_mut();
                        Self::render_search(
                            Box::new(acc_searcher),
                            ui,
                            self.search.clone(),
                            self.page.clone(),
                            false,
                        )
                    }
                    SearchType::Wiki => {
                        let wiki_searcher = self.wiki_searcher.borrow_mut();
                        Self::render_search(
                            Box::new(wiki_searcher),
                            ui,
                            self.search.clone(),
                            self.page.clone(),
                            true,
                        )
                    }
                }
            });
    }

    fn render_search<T>(
        searcher: Box<&dyn Searcher<Vec<T>>>,
        ui: &Ui,
        query: String,
        page: Arc<AtomicUsize>,
        hide_empty: bool,
    ) where
        T: Renderable,
    {
        let last_result = searcher.last_result();
        if searcher.is_loading() && hide_empty {
            for _ in 0..last_result.len() {
                ui.text("");
            }
        } else {
            let max_width = searcher
                .last_result()
                .iter()
                .map(|i| ui.calc_text_size(i.title())[0])
                .reduce(f32::max);

            for item in last_result.iter() {
                item.render_self(ui, max_width);
            }
        }

        if !searcher.last_result().is_empty() {
            Self::render_page_select(searcher, ui, query, page)
        }
    }

    fn render_page_select<T>(
        searcher: Box<&dyn Searcher<T>>,
        ui: &Ui,
        query: String,
        page: Arc<AtomicUsize>,
    ) {
        let curr = page.load(Ordering::SeqCst);
        if curr > 0 {
            if ui.arrow_button("##Prev", Direction::Left) {
                page.store(curr - 1, Ordering::SeqCst);
                searcher.search(query.clone(), page.load(Ordering::SeqCst));
            }
        } else {
            ui.dummy([20.0, 20.0])
        }

        ui.same_line();

        if searcher.has_more() {
            if ui.arrow_button("##More", Direction::Right) {
                page.store(curr + 1, Ordering::SeqCst);
                searcher.search(query.clone(), page.load(Ordering::SeqCst));
            }
        } else {
            ui.dummy([20.0, 20.0]);
        }
    }
}
