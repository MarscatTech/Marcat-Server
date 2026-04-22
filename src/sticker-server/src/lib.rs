#[macro_use]
extern crate ic_cdk_macros;

mod types;
mod storage;
mod auth;
mod upload;
mod zip_upload;

use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use types::{ChunkKey, Error, PackStickerKey, PackSummary, PacksPage, Sticker, StickerInfo, StickerPack, StorageType, StickersPage, UploadStatus};
use storage::{PACKS, PACK_STICKERS, STICKERS, CHUNKS};

#[derive(CandidType, Deserialize)]
struct HttpRequest {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

#[derive(CandidType, Serialize)]
struct HttpResponse {
    status_code: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

#[init]
fn init() {
    auth::init_admin(ic_cdk::caller());
}

#[post_upgrade]
fn post_upgrade() {
}

#[update]
fn add_admin(principal: Principal) -> Result<(), Error> {
    auth::require_admin()?;
    storage::ADMINS.with(|a| {
        a.borrow_mut().insert(principal, ());
    });
    Ok(())
}

#[update]
fn remove_admin(principal: Principal) -> Result<(), Error> {
    auth::require_admin()?;
    storage::ADMINS.with(|a| {
        a.borrow_mut().remove(&principal);
    });
    Ok(())
}

#[update]
fn create_pack(name: String, description: String) -> Result<u32, Error> {
    auth::require_admin()?;
    let id = storage::next_pack_id();
    let now = ic_cdk::api::time();
    let pack = StickerPack {
        id,
        name,
        description,
        cover_sticker_id: None,
        creator: ic_cdk::caller(),
        created_at: now,
        updated_at: now,
    };
    PACKS.with(|p| p.borrow_mut().insert(id, pack));
    Ok(id)
}

#[update]
fn update_pack(
    pack_id: u32,
    name: Option<String>,
    description: Option<String>,
    cover_sticker_id: Option<u64>,
) -> Result<(), Error> {
    auth::require_admin()?;
    PACKS.with(|p| {
        let mut packs = p.borrow_mut();
        let mut pack = packs.get(&pack_id).ok_or(Error::PackNotFound)?;
        if let Some(n) = name {
            pack.name = n;
        }
        if let Some(d) = description {
            pack.description = d;
        }
        if let Some(c) = cover_sticker_id {
            pack.cover_sticker_id = Some(c);
        }
        pack.updated_at = ic_cdk::api::time();
        packs.insert(pack_id, pack);
        Ok(())
    })
}

#[update]
fn delete_pack(pack_id: u32) -> Result<(), Error> {
    auth::require_admin()?;

    PACKS.with(|p| {
        if p.borrow().get(&pack_id).is_none() {
            return Err(Error::PackNotFound);
        }
        Ok(())
    })?;

    let sticker_ids: Vec<u32> = PACK_STICKERS.with(|ps| {
        let ps = ps.borrow();
        let start = PackStickerKey { pack_id, sticker_id: 0 };
        let end = PackStickerKey { pack_id, sticker_id: u32::MAX };
        ps.range(start..=end)
            .map(|(key, _)| key.sticker_id)
            .collect()
    });

    for sticker_id in &sticker_ids {
        delete_sticker_internal(*sticker_id, pack_id);
    }

    PACKS.with(|p| p.borrow_mut().remove(&pack_id));
    Ok(())
}

fn delete_sticker_internal(sticker_id: u32, pack_id: u32) {
    STICKERS.with(|s| {
        if let Some(sticker) = s.borrow().get(&sticker_id) {
            if let StorageType::Binary { chunk_count, .. } = &sticker.storage_type {
                CHUNKS.with(|c| {
                    let mut chunks = c.borrow_mut();
                    for i in 0..*chunk_count {
                        chunks.remove(&ChunkKey { sticker_id, chunk_index: i });
                    }
                });
            }
        }
    });
    STICKERS.with(|s| s.borrow_mut().remove(&sticker_id));
    PACK_STICKERS.with(|ps| {
        ps.borrow_mut().remove(&PackStickerKey { pack_id, sticker_id });
    });
}

#[query]
fn get_pack(pack_id: u32) -> Result<StickerPack, Error> {
    PACKS.with(|p| {
        p.borrow().get(&pack_id).ok_or(Error::PackNotFound)
    })
}

#[query]
fn list_packs(offset: u32, limit: u32) -> PacksPage {
    let total = PACKS.with(|p| p.borrow().len() as u32);
    let packs = PACKS.with(|p| {
        p.borrow()
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .map(|(_, pack)| {
                let sticker_count = count_pack_stickers(pack.id);
                PackSummary {
                    id: pack.id,
                    name: pack.name.clone(),
                    description: pack.description.clone(),
                    cover_sticker_id: pack.cover_sticker_id,
                    creator: pack.creator,
                    created_at: pack.created_at,
                    updated_at: pack.updated_at,
                    sticker_count,
                }
            })
            .collect()
    });
    PacksPage { packs, total }
}

#[update]
fn create_sticker(
    pack_id: u32,
    name: String,
    tags: Vec<String>,
    storage_type: StorageType,
    data: Option<Vec<u8>>,
) -> Result<u32, Error> {
    auth::require_admin()?;

    PACKS.with(|p| {
        if p.borrow().get(&pack_id).is_none() {
            return Err(Error::PackNotFound);
        }
        Ok(())
    })?;

    if let StorageType::Binary { ref content_type, .. } = storage_type {
        upload::validate_content_type(content_type)?;
    }

    let upload_status = match (&storage_type, &data) {
        (StorageType::Binary { .. }, Some(_)) => UploadStatus::Complete,
        (StorageType::Binary { .. }, None) => UploadStatus::Pending { uploaded_chunks: vec![] },
        (StorageType::External { .. }, _) => UploadStatus::Complete,
    };

    let id = storage::next_sticker_id();
    let now = ic_cdk::api::time();

    let sticker = Sticker {
        id,
        pack_id,
        name,
        tags,
        storage_type,
        upload_status,
        width: None,
        height: None,
        created_at: now,
        updated_at: now,
    };

    STICKERS.with(|s| s.borrow_mut().insert(id, sticker));
    PACK_STICKERS.with(|ps| {
        ps.borrow_mut().insert(PackStickerKey { pack_id, sticker_id: id }, ());
    });

    if let Some(d) = data {
        upload::store_single_chunk(id, d)?;
    }

    Ok(id)
}

#[update]
fn update_sticker(
    sticker_id: u32,
    name: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<(), Error> {
    auth::require_admin()?;
    STICKERS.with(|s| {
        let mut stickers = s.borrow_mut();
        let mut sticker = stickers.get(&sticker_id).ok_or(Error::StickerNotFound)?;
        if let Some(n) = name {
            sticker.name = n;
        }
        if let Some(t) = tags {
            sticker.tags = t;
        }
        sticker.updated_at = ic_cdk::api::time();
        stickers.insert(sticker_id, sticker);
        Ok(())
    })
}

#[update]
fn delete_sticker(sticker_id: u32) -> Result<(), Error> {
    auth::require_admin()?;
    let pack_id = STICKERS.with(|s| {
        s.borrow().get(&sticker_id)
            .map(|st| st.pack_id)
            .ok_or(Error::StickerNotFound)
    })?;
    delete_sticker_internal(sticker_id, pack_id);
    Ok(())
}

#[update]
fn upload_chunk(sticker_id: u32, chunk_index: u32, data: Vec<u8>) -> Result<(), Error> {
    auth::require_admin()?;
    upload::upload_chunk(sticker_id, chunk_index, data)
}

#[update]
fn finalize_upload(sticker_id: u32) -> Result<(), Error> {
    auth::require_admin()?;
    upload::finalize_upload(sticker_id)
}

#[query]
fn get_sticker(sticker_id: u32) -> Result<Sticker, Error> {
    STICKERS.with(|s| {
        s.borrow().get(&sticker_id).ok_or(Error::StickerNotFound)
    })
}

#[query]
fn list_stickers(pack_id: u32, offset: u32, limit: u32) -> StickersPage {
    let total = count_pack_stickers(pack_id);
    let sticker_ids: Vec<u32> = PACK_STICKERS.with(|ps| {
        let ps = ps.borrow();
        let start = PackStickerKey { pack_id, sticker_id: 0 };
        let end = PackStickerKey { pack_id, sticker_id: u32::MAX };
        ps.range(start..=end)
            .skip(offset as usize)
            .take(limit as usize)
            .map(|(key, _)| key.sticker_id)
            .collect()
    });

    let stickers = STICKERS.with(|s| {
        let stickers = s.borrow();
        sticker_ids.iter()
            .filter_map(|id| stickers.get(id).map(|st| StickerInfo {
                id: st.id,
                pack_id: st.pack_id,
                name: st.name.clone(),
                storage_type: st.storage_type.clone(),
                created_at: st.created_at,
                updated_at: st.updated_at,
            }))
            .collect()
    });
    StickersPage { stickers, total }
}

#[query]
fn get_chunk(sticker_id: u32, chunk_index: u32) -> Result<Vec<u8>, Error> {
    CHUNKS.with(|c| {
        c.borrow()
            .get(&ChunkKey { sticker_id, chunk_index })
            .ok_or(Error::StickerNotFound)
    })
}

#[query]
fn get_sticker_data(sticker_id: u32) -> Result<Vec<u8>, Error> {
    upload::get_sticker_data(sticker_id)
}

fn count_pack_stickers(pack_id: u32) -> u32 {
    PACK_STICKERS.with(|ps| {
        let ps = ps.borrow();
        let start = PackStickerKey { pack_id, sticker_id: 0 };
        let end = PackStickerKey { pack_id, sticker_id: u32::MAX };
        ps.range(start..=end).count() as u32
    })
}

fn find_pack_by_name(name: &str) -> Option<StickerPack> {
    PACKS.with(|p| {
        p.borrow()
            .iter()
            .find(|(_, pack)| pack.name == name)
            .map(|(_, pack)| pack)
    })
}

fn get_pack_sticker_by_index(pack_id: u32, one_based: usize) -> Option<u32> {
    PACK_STICKERS.with(|ps| {
        let ps = ps.borrow();
        let start = PackStickerKey { pack_id, sticker_id: 0 };
        let end = PackStickerKey { pack_id, sticker_id: u32::MAX };
        ps.range(start..=end)
            .nth(one_based - 1)
            .map(|(key, _)| key.sticker_id)
    })
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[query]
fn http_request(req: HttpRequest) -> HttpResponse {
    let path = req.url.split('?').next().unwrap_or(&req.url);

    if let Some(id_str) = path.strip_prefix("/sticker/") {
        if let Ok(sticker_id) = id_str.parse::<u32>() {
            return serve_sticker(sticker_id);
        }
    }

    let trimmed = path.trim_start_matches('/');
    let parts: Vec<&str> = trimmed.splitn(2, '/').collect();
    if parts.len() == 2 {
        let pack_name = percent_decode(parts[0]);
        if let Ok(index) = parts[1].parse::<usize>() {
            if index >= 1 {
                if let Some(pack) = find_pack_by_name(&pack_name) {
                    if let Some(sticker_id) = get_pack_sticker_by_index(pack.id, index) {
                        return serve_sticker(sticker_id);
                    }
                }
                return HttpResponse {
                    status_code: 404,
                    headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
                    body: b"Sticker not found".to_vec(),
                };
            }
        }
    }

    HttpResponse {
        status_code: 404,
        headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
        body: b"Not Found. Use /sticker/{id} or /{pack_name}/{index}".to_vec(),
    }
}

fn serve_sticker(sticker_id: u32) -> HttpResponse {
    let sticker = match STICKERS.with(|s| s.borrow().get(&sticker_id)) {
        Some(s) => s,
        None => {
            return HttpResponse {
                status_code: 404,
                headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
                body: b"Sticker not found".to_vec(),
            };
        }
    };

    match &sticker.storage_type {
        StorageType::Binary { content_type, chunk_count, .. } => {
            if !matches!(sticker.upload_status, UploadStatus::Complete) {
                return HttpResponse {
                    status_code: 404,
                    headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
                    body: b"Upload not complete".to_vec(),
                };
            }

            let mut data = Vec::new();
            CHUNKS.with(|c| {
                let chunks = c.borrow();
                for i in 0..*chunk_count {
                    if let Some(chunk) = chunks.get(&ChunkKey { sticker_id, chunk_index: i }) {
                        data.extend_from_slice(&chunk);
                    }
                }
            });

            HttpResponse {
                status_code: 200,
                headers: vec![
                    ("Content-Type".to_string(), content_type.clone()),
                    ("Cache-Control".to_string(), "public, max-age=604800".to_string()),
                ],
                body: data,
            }
        }
        StorageType::External { url } => {
            HttpResponse {
                status_code: 302,
                headers: vec![("Location".to_string(), url.clone())],
                body: vec![],
            }
        }
    }
}


#[update]
fn upload_zip(pack_id: u32, zip_data: Vec<u8>) -> Result<Vec<u32>, Error> {
    auth::require_admin()?;

    PACKS.with(|p| {
        if p.borrow().get(&pack_id).is_none() {
            return Err(Error::PackNotFound);
        }
        Ok(())
    })?;

    zip_upload::create_pack_from_zip(pack_id, zip_data)
}
