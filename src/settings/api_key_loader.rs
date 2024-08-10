use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::entities::{Gw2ApiKey, Gw2Permission, LoadingState};
use crate::{utils, THREADS};

#[derive(Debug, Clone)]
pub struct ApiKeyLoader {
    last_update: Instant,
    state: usize,
    states: [&'static str; 4],
    loading_state: Arc<Mutex<LoadingState<String>>>,
}

impl Default for ApiKeyLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiKeyLoader {
    pub fn new() -> Self {
        Self {
            last_update: Instant::now(),
            state: 0,
            states: ["", ".", "..", "..."],
            loading_state: Arc::new(Mutex::new(LoadingState::Init)),
        }
    }

    pub fn loading_state(&self) -> LoadingState<String> {
        self.loading_state.clone().lock().unwrap().clone()
    }

    pub fn verify_api_key(&mut self, check_api_key: String) {
        if *self.loading_state.lock().unwrap() == LoadingState::Loading {
            return;
        }

        let loading_state = self.loading_state.clone();
        unsafe {
            THREADS.get_mut().unwrap().push(thread::spawn(move || {
                *loading_state.lock().unwrap() = LoadingState::Loading;
                let result = utils::request::<Gw2ApiKey>(check_api_key.clone(), "tokeninfo");
                // Shhhhhhh don't tell anyone
                thread::sleep(Duration::from_millis(500));

                match result {
                    Err(_) => {
                        *loading_state.lock().unwrap() = LoadingState::Error("Invalid Api Key!");
                    }
                    Ok(api_key) => {
                        if api_key.permissions.contains(&Gw2Permission::Inventories) && api_key.permissions.contains(&Gw2Permission::Account) {
                            *loading_state.lock().unwrap() =
                                LoadingState::Success(check_api_key.clone())
                        } else {
                            *loading_state.lock().unwrap() =
                                LoadingState::Error("Invalid permissions!");
                        }
                    }
                }
            }));
        }
    }

    pub fn update(&mut self) {
        if self.last_update.elapsed() >= Duration::from_millis(500) {
            self.state = (self.state + 1) % self.states.len();
            self.last_update = Instant::now();
        }
    }

    pub fn curr_dots(&self) -> &str {
        self.states[self.state]
    }
}
