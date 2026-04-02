#![allow(clippy::collapsible_else_if)]

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use candid::Principal;
use ic_cdk_macros::{query, update, init, post_upgrade};

mod code;
mod types;

use types::{
    Announcement, PaginatedAnnouncements, StringList, VersionUpdate, _add_api_key,
    _delete_announcement, _get_announcements_by_key_paginated, _get_api_key, _get_version_by_key,
    _insert_announcement, _update_version, OWNER, _delete_api_key
};
use crate::types::{AppDownloadRecord, AppVersion, DownloadInfo, Info, ReviewVersionRequest, ReviewVersionResponse, TopDownload, _check_owner, _check_review_version, _get_app_version, _set_app_version, _set_auth_message, _set_auth_secret, _verify_signature, DOWNLOAD_COUNT_INFO};
use candid::CandidType;
use serde::Deserialize;

#[derive(CandidType, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(CandidType)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub upgrade: Option<bool>,
}

#[init]
fn init() {
    let owner = ic_cdk::caller();
    OWNER.with(|store| {
        store.borrow_mut().insert("owner".to_string(), owner);
    });
}


#[post_upgrade]
fn post_upgrade() {
    OWNER.with(|store| {
        let mut store = store.borrow_mut();
        if store.get(&"owner".to_string()).is_none() {
            store.insert("owner".to_string(), ic_cdk::caller());
        }
    });
}

#[query]
pub fn owner(signature: String) -> Result<Option<Principal>, u64> {
    _verify_signature(&signature)?;
    Ok(OWNER.with(|store| {
        store.borrow().get(&"owner".to_string())
    }))
}

#[update]
pub fn update_version(version: VersionUpdate) -> Result<(), u64> {
    _update_version(1, version)?;
    Ok(())
}

#[query]
pub fn get_version() -> Result<VersionUpdate, u64> {
    _get_version_by_key(1)
}

#[update]
pub fn add_announcement(
    language: String,
    announcement_id: u64,
    announcement: Announcement,
) -> Result<(), u64> {
    _insert_announcement(language, announcement_id, announcement)
}

#[update]
pub fn remove_announcement(language: String, announcement_id: u64) -> Result<(), u64> {
    _delete_announcement(language, announcement_id)
}

#[query]
pub fn announcements(
    key: String,
    page_number: usize,
    page_size: usize,
) -> Result<PaginatedAnnouncements, u64> {
    _get_announcements_by_key_paginated(key, page_number, page_size)
}

#[update]
pub fn set_api_key(key: String, values: Vec<String>) -> Result<(), u64> {
    _add_api_key(key, values)
}

#[update]
pub fn remove_api_key(key: String) -> Result<(), u64> {
    _delete_api_key(key)
}

#[query]
pub fn api_key(key: String, signature: String) -> Result<StringList, u64> {
    _verify_signature(&signature)?;
    _get_api_key(key, signature)
}

#[query]
fn time_millis() -> u64 {
    ic_cdk::api::time() / 1_000_000
}

#[update]
pub fn set_info(
    app_address: String,
    name: String,
    about: String,
    version: String,
    update_time: u64,
    download_count: u64,
    flow: u64
) -> Result<(), String> {
    _check_owner()?;
    DOWNLOAD_COUNT_INFO.with(|map| {
        let mut map = map.borrow_mut();

        if let Some(mut existing) = map.get(&app_address) {
            existing.info.name = name;
            existing.info.about = about;
            existing.info.version = version;
            existing.info.update_time = update_time;

            existing.download.download_count = download_count;
            existing.download.flow = flow;

            map.insert(app_address, existing);
            return Ok(());
        }

        let record = AppDownloadRecord {
            info: Info {
                name,
                about,
                version,
                update_time,
            },
            download: DownloadInfo {
                download_count,
                flow,
            },
        };

        map.insert(app_address, record);
        Ok(())
    })
}


#[query]
pub fn top_info(n: usize) -> Result<Vec<TopDownload>, String> {
    if n == 0 {
        return Ok(vec![]);
    }

    let mut heap: BinaryHeap<Reverse<(u64, String)>> = BinaryHeap::new();

    DOWNLOAD_COUNT_INFO.with(|map| {
        let map = map.borrow();

        for (app_addr, record) in map.iter() {
            let count = record.download.download_count;

            let entry = Reverse((count, app_addr.clone()));

            if heap.len() < n {
                heap.push(entry);
            } else if let Some(Reverse((min_count, _))) = heap.peek() {
                if count > *min_count {
                    heap.pop();
                    heap.push(entry);
                }
            }
        }
    });

    let mut sorted: Vec<(u64, String)> =
        heap.into_iter().map(|Reverse(x)| x).collect();

    sorted.sort_by(|a, b| b.0.cmp(&a.0));

    let mut result = Vec::with_capacity(sorted.len());

    for (_count, app_addr) in sorted {
        let rec_opt = DOWNLOAD_COUNT_INFO.with(|map| map.borrow().get(&app_addr));

        if let Some(record) = rec_opt {
            result.push(TopDownload {
                app_address: app_addr,
                info: record.info.clone(),
            });
        }
    }

    Ok(result)
}

#[query]
fn http_request(req: HttpRequest) -> HttpResponse {
    let path = req.url.split('?').next().unwrap_or("");

    match path {
        "/get_version" => {
            match _get_version_by_key(1) {
                Ok(version) => {
                    let body = serde_json::to_vec(&version).unwrap_or_default();
                    HttpResponse {
                        status_code: 200,
                        headers: vec![
                            ("Content-Type".to_string(), "application/json".to_string()),
                            ("Access-Control-Allow-Origin".to_string(), "*".to_string()),
                        ],
                        body,
                        upgrade: Some(false),
                    }
                }
                Err(e) => {
                    let body = format!("{{\"error\": {}}}", e).into_bytes();
                    HttpResponse {
                        status_code: 404,
                        headers: vec![
                            ("Content-Type".to_string(), "application/json".to_string()),
                        ],
                        body,
                        upgrade: Some(false),
                    }
                }
            }
        }

        _ => HttpResponse {
            status_code: 404,
            headers: vec![],
            body: b"Not Found".to_vec(),
            upgrade: Some(false),
        },
    }
}

#[update]
pub fn set_auth_secret(secret: String) -> Result<(), u64> {
    _set_auth_secret(secret)
}

#[update]
pub fn set_auth_message(message: String) -> Result<(), u64> {
    _set_auth_message(message)
}

#[query]
pub fn check_review_version(req: ReviewVersionRequest) -> Result<ReviewVersionResponse, u64> {
    _check_review_version(req)
}

#[update]
pub fn set_app_version(version: AppVersion) -> Result<(), u64> {
    _set_app_version(version)
}

#[query]
pub fn get_app_version() -> Result<AppVersion, u64> {
    _get_app_version()
}

ic_cdk::export_candid!();
