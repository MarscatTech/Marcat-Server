use candid::{CandidType, Principal};
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    storable::Bound,
    DefaultMemoryImpl, StableBTreeMap, Storable,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
type Memory = VirtualMemory<DefaultMemoryImpl>;
use crate::code::error_code::*;
use std::borrow::Cow;
use ciborium::{from_reader, into_writer};
use hmac::{Hmac, Mac};
use sha2::Sha256;
type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Serialize, Deserialize, CandidType)]
pub struct VersionUpdate {
    ios_version: String,
    ios_title: String,
    ios_download_url: String,
    android_version: String,
    android_title: String,
    android_download_url: String,
    force_update: bool,
    description: String,
    release_time: u64,
}

impl Storable for VersionUpdate {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buf = vec![];
        serde_json::to_writer(&mut buf, self).expect("Failed to serialize VersionUpdate");
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        serde_json::from_slice(&bytes).expect("Failed to deserialize VersionUpdate")
    }
}

pub fn _update_version(version_id: u64, version: VersionUpdate) -> Result<(), u64> {
    _check_owner().map_err(|_| PERMISSION_NOT_FOUND)?;

    VERSION_UPDATES.with(|storage| {
        let mut storage = storage.borrow_mut();
        storage.insert(version_id, version);
        Ok(())
    })
}

pub fn _get_version_by_key(key: u64) -> Result<VersionUpdate, u64> {
    VERSION_UPDATES.with(|map| {
        let map = map.borrow();
        if let Some(value) = map.get(&key) {
            Ok(value.clone())
        } else {
            Err(DATA_NOT_EXISTS)
        }
    })
}

#[derive(Clone, Serialize, Deserialize, CandidType)]
pub struct Announcement {
    pub title: String,
    pub content: String,
    pub timestamp: u64,
    pub url: String,
}

#[derive(Clone, Serialize, Deserialize, CandidType)]
pub struct AnnouncementWithId {
    pub id: u64,
    pub title: String,
    pub content: String,
    pub timestamp: u64,
    pub url: String,
}

#[derive(Default, Clone)]
pub struct AnnouncementList(pub Vec<(u64, Announcement)>);

#[derive(Clone, Serialize, Deserialize, CandidType)]
pub struct PaginatedAnnouncements {
    pub announcements: Vec<AnnouncementWithId>,
    pub total_announcements: usize,
    pub total_pages: usize,
}

impl Storable for Announcement {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buf = vec![];
        serde_json::to_writer(&mut buf, self).expect("Failed to serialize Announcement");
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        serde_json::from_slice(&bytes).expect("Failed to deserialize Announcement")
    }
}

impl Storable for AnnouncementList {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buf = vec![];
        serde_json::to_writer(&mut buf, &self.0).expect("Failed to serialize AnnouncementList");
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let inner = serde_json::from_slice(&bytes).expect("Failed to deserialize AnnouncementList");
        AnnouncementList(inner)
    }
}

pub fn _insert_announcement(
    language: String,
    announcement_id: u64,
    announcement: Announcement,
) -> Result<(), u64> {
    _check_owner().map_err(|_| PERMISSION_NOT_FOUND)?;

    ANNOUNCEMENTS.with(|storage| {
        let mut storage = storage.borrow_mut();
        let mut announcement_list = storage.get(&language).unwrap_or(AnnouncementList(vec![]));
        if announcement_list
            .0
            .iter()
            .any(|(id, _)| *id == announcement_id)
        {
            return Err(DATA_ALREADY_EXISTS);
        }
        announcement_list.0.push((announcement_id, announcement));
        storage.insert(language, announcement_list);
        Ok(())
    })
}

pub fn _delete_announcement(language: String, announcement_id: u64) -> Result<(), u64> {
    _check_owner().map_err(|_| PERMISSION_NOT_FOUND)?;

    ANNOUNCEMENTS.with(|storage| {
        let mut storage = storage.borrow_mut();
        let mut announcement_list = storage.get(&language).ok_or(DATA_NOT_EXISTS)?;
        let original_len = announcement_list.0.len();
        announcement_list.0.retain(|(id, _)| *id != announcement_id);
        if announcement_list.0.len() == original_len {
            return Err(DATA_NOT_EXISTS);
        }
        storage.insert(language, announcement_list);
        Ok(())
    })
}

pub fn _get_announcements_by_key_paginated(
    language: String,
    page: usize,
    page_size: usize,
) -> Result<PaginatedAnnouncements, u64> {
    ANNOUNCEMENTS.with(|announcements| {
        let storage = announcements.borrow();
        if let Some(mut list) = storage.get(&language) {
            list.0.sort_by(|a, b| b.0.cmp(&a.0));

            let total_announcements = list.0.len();
            let total_pages = (total_announcements + page_size - 1) / page_size;

            let start = page * page_size;
            if start >= total_announcements {
                return Ok(PaginatedAnnouncements {
                    announcements: vec![],
                    total_announcements,
                    total_pages,
                });
            }

            let announcements = list
                .0
                .iter()
                .skip(start)
                .take(page_size)
                .map(|(id, announcement)| AnnouncementWithId {
                    id: *id,
                    title: announcement.title.clone(),
                    content: announcement.content.clone(),
                    timestamp: announcement.timestamp.clone(),
                    url: announcement.url.clone(),
                })
                .collect();

            Ok(PaginatedAnnouncements {
                announcements,
                total_announcements,
                total_pages,
            })
        } else {
            Err(DATA_NOT_EXISTS)
        }
    })
}

#[derive(Clone, Serialize, Deserialize, CandidType)]
pub struct StringList(pub Vec<String>);

impl Storable for StringList {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buf = vec![];
        serde_json::to_writer(&mut buf, &self.0).expect("Failed to serialize StringList");
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let inner = serde_json::from_slice(&bytes).expect("Failed to deserialize StringList");
        StringList(inner)
    }
}

pub fn _add_api_key(key: String, values: Vec<String>) -> Result<(), u64> {
    _check_owner().map_err(|_| PERMISSION_NOT_FOUND)?;

    API_KEYS.with(|map| {
        let mut map = map.borrow_mut();
        let string_list = StringList(values);
        map.insert(key, string_list);
        Ok(())
    })
}

pub fn _delete_api_key(key: String) -> Result<(), u64> {
    _check_owner().map_err(|_| PERMISSION_NOT_FOUND)?;
    API_KEYS.with( |map| {
        let mut map = map.borrow_mut();
        if map.remove(&key).is_some()  {
            Ok(())
        } else {
            Err(DATA_NOT_EXISTS)
        }
    })
}

pub fn _get_api_key(key: String, signature: String) -> Result<StringList, u64> {
    API_KEYS.with(|map| {
        let map = map.borrow();
        if let Some(string_list) = map.get(&key) {
            Ok(string_list.clone())
        } else {
            Err(DATA_NOT_EXISTS)
        }
    })
}


pub fn _check_owner() -> Result<(), String> {
    let caller = ic_cdk::caller();
    OWNER.with(|store| {
        let store = store.borrow();
        match store.get(&"owner".to_string()) {
            Some(owner) if owner == caller => Ok(()),
            _ => Err("Only owner can perform this action".to_string()),
        }
    })
}

pub fn _set_auth_secret(secret: String) -> Result<(), u64> {
    _check_owner().map_err(|_| PERMISSION_NOT_FOUND)?;
    AUTH_SECRET.with(|s| {
        s.borrow_mut().insert("secret".to_string(), secret);
    });
    Ok(())
}

pub fn _set_auth_message(message: String) -> Result<(), u64> {
    _check_owner().map_err(|_| PERMISSION_NOT_FOUND)?;
    AUTH_SECRET.with(|s| {
        s.borrow_mut().insert("message".to_string(), message);
    });
    Ok(())
}

pub fn _verify_signature(signature: &str) -> Result<(), u64> {
    let secret = AUTH_SECRET.with(|s| {
        s.borrow().get(&"secret".to_string())
    }).ok_or(PERMISSION_NOT_FOUND)?;

    let message = AUTH_SECRET.with(|s| {
        s.borrow().get(&"message".to_string())
    }).ok_or(PERMISSION_NOT_FOUND)?;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| PERMISSION_NOT_FOUND)?;
    mac.update(message.as_bytes());
    let result = mac.finalize().into_bytes();
    let expected = hex::encode(result);

    if constant_time_eq(&expected, signature) {
        Ok(())
    } else {
        Err(PERMISSION_NOT_FOUND)
    }
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

pub fn _get_app_version() -> Result<AppVersion, u64> {
    APP_VERSION.with(|map| {
        let map = map.borrow();
        if let Some(value) = map.get(&1u64) {
            Ok(value.clone())
        } else {
            Err(DATA_NOT_EXISTS)
        }
    })
}

pub fn _set_app_version(version: AppVersion) -> Result<(), u64> {
    _check_owner().map_err(|_| PERMISSION_NOT_FOUND)?;
    APP_VERSION.with(|map| {
        let mut map = map.borrow_mut();
        map.insert(1u64, version);
        Ok(())
    })
}

pub fn _check_review_version(req: ReviewVersionRequest) -> Result<ReviewVersionResponse, u64> {
    let current = _get_app_version()?;

    Ok(ReviewVersionResponse {
        ios_h: match req.ios_version {
            Some(v) if v == current.ios_version => 1,
            _ => 0,
        },
        android_h: match req.android_version {
            Some(v) if v == current.android_version => 1,
            _ => 0,
        },
    })
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct Info {
    pub name : String,
    pub about: String,
    pub version: String,
    pub update_time: u64,
}

#[derive(Clone, CandidType, Deserialize, Serialize, Debug, Default)]
pub struct DownloadInfo {
    pub download_count: u64,
    pub flow: u64,
}


#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct TopDownload {
    pub app_address: String,
    pub info: Info,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct AppDownloadRecord {
    pub info: Info,
    pub download: DownloadInfo,
}

impl Storable for AppDownloadRecord {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let mut buf = vec![];
        into_writer(self, &mut buf).expect("failed to encode AppDownloadRecord");
        buf.into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        from_reader(&bytes[..]).expect("failed to decode AppDownloadRecord")
    }
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct ReviewVersionRequest {
    pub ios_version: Option<String>,
    pub android_version: Option<String>,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct ReviewVersionResponse {
    pub ios_h: u8,
    pub android_h: u8,
}


#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct AppVersion {
    pub android_version: String,
    pub ios_version: String,
}

impl Storable for AppVersion {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buf = vec![];
        serde_json::to_writer(&mut buf, self).expect("Failed to serialize AppVersion");
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        serde_json::from_slice(&bytes).expect("Failed to deserialize AppVersion")
    }
}

thread_local! {
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
            RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    pub static VERSION_UPDATES: RefCell<StableBTreeMap<u64, VersionUpdate, Memory>> =
      RefCell::new(StableBTreeMap::init(
                MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))),
            ));

    pub static ANNOUNCEMENTS: RefCell<StableBTreeMap<String, AnnouncementList, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))),
        ));

    pub static NEXT_MEM_ID_ANNOUNCE: RefCell<u8> = const { RefCell::new(100) };

    pub static OWNER: RefCell<StableBTreeMap<String, Principal, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2))),
        ));

    pub static API_KEYS: RefCell<StableBTreeMap<String, StringList, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3)))
        )
    );

    pub static DOWNLOAD_COUNT_INFO: RefCell<StableBTreeMap<String, AppDownloadRecord, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(4))),
        ));

    pub static AUTH_SECRET: RefCell<StableBTreeMap<String, String, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(5))),
        ));

    pub static APP_VERSION: RefCell<StableBTreeMap<u64, AppVersion, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(6))),
    ));
}

