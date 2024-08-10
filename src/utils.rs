use std::convert::identity;
use std::path::PathBuf;
use std::time::Duration;

use nexus::paths::get_addon_dir;
use serde::de::DeserializeOwned;
use ureq::Error;

use crate::entities::Gw2Item;
use crate::settings::settings::Settings;

pub fn request<T: DeserializeOwned>(api_key: String, endpoint: &str) -> anyhow::Result<T> {
    let mut authorization = "Bearer ".to_string();
    authorization.push_str(api_key.as_str());

    let mut url = "https://api.guildwars2.com/v2/".to_string();
    url.push_str(endpoint);

    match ureq::get(url.as_str())
        .set("Authorization", &authorization)
        .call()
    {
        Ok(response) => Ok(response.into_json::<T>()?),
        Err(e) => match e {
            Error::Status(code, _) => {
                if code == 429 {
                    std::thread::sleep(Duration::from_millis(500));
                    request(api_key, endpoint)
                } else {
                    Err(e)?
                }
            }
            Error::Transport(_) => Err(e)?,
        },
    }
}

pub fn auth_request<T: DeserializeOwned>(endpoint: &str) -> anyhow::Result<T> {
    let settings = Settings::get();
    request(settings.api_key.clone(), endpoint)
}

pub fn fetch_items(ids: Vec<usize>) -> Vec<Gw2Item> {
    ids.chunks(200)
        .map(|ids| {
            let id_str = ids
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",");

            let url = format!("items?lang=en&ids={id_str}");

            auth_request::<Vec<Gw2Item>>(url.as_str())
        })
        .filter_map(|i| i.ok())
        .flat_map(identity)
        .collect()
}

pub unsafe fn sub_path(sub_dir: &str) -> PathBuf {
    get_addon_dir("find-my-sht")
        .expect("addon dir to exist")
        .join(sub_dir)
}
