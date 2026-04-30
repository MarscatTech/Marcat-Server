use crate::storage::{self, CHUNKS, PACK_STICKERS, STICKERS};
use crate::types::{
    extension_to_content_type, ChunkKey, Error, PackStickerKey, Sticker, StorageType,
    UploadStatus, MAX_CHUNK_SIZE, MAX_ZIP_IMAGES, MAX_ZIP_SIZE,
};
use std::io::{Cursor, Read};
use zip::ZipArchive;

pub fn create_pack_from_zip(
    pack_id: u32,
    zip_data: Vec<u8>,
) -> Result<Vec<u32>, Error> {
    if zip_data.len() > MAX_ZIP_SIZE {
        return Err(Error::ZipTooLarge);
    }

    let cursor = Cursor::new(zip_data);
    let mut archive = ZipArchive::new(cursor).map_err(|e| Error::ZipExtractFailed {
        detail: format!("Failed to open zip: {}", e),
    })?;

    let mut image_entries: Vec<(String, String)> = Vec::new();
    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| Error::ZipExtractFailed {
            detail: format!("Failed to read zip entry {}: {}", i, e),
        })?;

        let name = file.name().to_string();
        if file.is_dir() || name.starts_with("__MACOSX") || name.starts_with('.') {
            continue;
        }

        if let Some(ext) = name.rsplit('.').next() {
            if let Some(content_type) = extension_to_content_type(ext) {
                image_entries.push((name, content_type.to_string()));
            }
        }
    }

    if image_entries.is_empty() {
        return Err(Error::ZipNoImagesFound);
    }
    if image_entries.len() > MAX_ZIP_IMAGES {
        return Err(Error::ZipTooManyImages);
    }

    let cursor = Cursor::new(archive.into_inner().into_inner());
    let mut archive = ZipArchive::new(cursor).map_err(|e| Error::ZipExtractFailed {
        detail: format!("Failed to reopen zip: {}", e),
    })?;

    let mut sticker_ids = Vec::new();

    for (filename, content_type) in &image_entries {
        let mut file = archive.by_name(filename).map_err(|e| Error::ZipExtractFailed {
            detail: format!("Failed to read {}: {}", filename, e),
        })?;

        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err(|e| Error::ZipExtractFailed {
            detail: format!("Failed to extract {}: {}", filename, e),
        })?;

        let size = data.len() as u64;
        let chunk_count = ((data.len() + MAX_CHUNK_SIZE - 1) / MAX_CHUNK_SIZE) as u32;

        let sticker_id = storage::next_sticker_id();
        let now = ic_cdk::api::time();

        let sticker_name = filename
            .rsplit('/')
            .next()
            .unwrap_or(filename)
            .rsplit('.')
            .last()
            .unwrap_or(filename)
            .to_string();

        let sticker = Sticker {
            id: sticker_id,
            pack_id,
            name: sticker_name,
            tags: vec![],
            storage_type: StorageType::Binary {
                content_type: content_type.clone(),
                size,
                chunk_count,
            },
            upload_status: UploadStatus::Complete,
            width: None,
            height: None,
            created_at: now,
            updated_at: now,
        };

        STICKERS.with(|s| s.borrow_mut().insert(sticker_id, sticker));

        PACK_STICKERS.with(|ps| {
            ps.borrow_mut().insert(
                PackStickerKey { pack_id, sticker_id },
                (),
            );
        });

        CHUNKS.with(|c| {
            let mut chunks = c.borrow_mut();
            for (i, chunk_data) in data.chunks(MAX_CHUNK_SIZE).enumerate() {
                chunks.insert(
                    ChunkKey {
                        sticker_id,
                        chunk_index: i as u32,
                    },
                    chunk_data.to_vec(),
                );
            }
        });

        sticker_ids.push(sticker_id);
    }

    Ok(sticker_ids)
}
