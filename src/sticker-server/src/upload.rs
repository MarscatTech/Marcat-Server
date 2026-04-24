use crate::storage::{CHUNKS, STICKERS};
use crate::types::{ChunkKey, Error, StorageType, UploadStatus, MAX_CHUNK_SIZE, ALLOWED_CONTENT_TYPES};

pub fn parse_image_size(data: &[u8]) -> Option<(u32, u32)> {
    imagesize::blob_size(data)
        .ok()
        .map(|s| (s.width as u32, s.height as u32))
}

pub fn validate_content_type(content_type: &str) -> Result<(), Error> {
    if ALLOWED_CONTENT_TYPES.contains(&content_type) {
        Ok(())
    } else {
        Err(Error::InvalidContentType)
    }
}

pub fn store_single_chunk(sticker_id: u32, data: Vec<u8>) -> Result<Option<(u32, u32)>, Error> {
    if data.len() > MAX_CHUNK_SIZE {
        return Err(Error::DataTooLarge);
    }
    let dims = parse_image_size(&data);
    CHUNKS.with(|c| {
        c.borrow_mut().insert(
            ChunkKey { sticker_id, chunk_index: 0 },
            data,
        );
    });
    Ok(dims)
}

pub fn upload_chunk(sticker_id: u32, chunk_index: u32, data: Vec<u8>) -> Result<(), Error> {
    if data.len() > MAX_CHUNK_SIZE {
        return Err(Error::DataTooLarge);
    }

    STICKERS.with(|s| {
        let mut stickers = s.borrow_mut();
        let mut sticker = stickers.get(&sticker_id).ok_or(Error::StickerNotFound)?;

        let uploaded_chunks = match &sticker.upload_status {
            UploadStatus::Pending { uploaded_chunks } => uploaded_chunks.clone(),
            UploadStatus::Complete => return Err(Error::UploadNotPending),
        };

        let chunk_count = match &sticker.storage_type {
            StorageType::Binary { chunk_count, .. } => *chunk_count,
            StorageType::External { .. } => return Err(Error::UploadNotPending),
        };
        if chunk_index >= chunk_count {
            return Err(Error::ChunkIndexOutOfRange);
        }
        if uploaded_chunks.contains(&chunk_index) {
            return Err(Error::ChunkAlreadyUploaded);
        }

        CHUNKS.with(|c| {
            c.borrow_mut().insert(
                ChunkKey { sticker_id, chunk_index },
                data,
            );
        });

        let mut new_uploaded = uploaded_chunks;
        new_uploaded.push(chunk_index);
        sticker.upload_status = UploadStatus::Pending { uploaded_chunks: new_uploaded };
        sticker.updated_at = ic_cdk::api::time();
        stickers.insert(sticker_id, sticker);

        Ok(())
    })
}

pub fn finalize_upload(sticker_id: u32) -> Result<(), Error> {
    STICKERS.with(|s| {
        let mut stickers = s.borrow_mut();
        let mut sticker = stickers.get(&sticker_id).ok_or(Error::StickerNotFound)?;

        let uploaded_chunks = match &sticker.upload_status {
            UploadStatus::Pending { uploaded_chunks } => uploaded_chunks.clone(),
            UploadStatus::Complete => return Err(Error::UploadNotPending),
        };

        let chunk_count = match &sticker.storage_type {
            StorageType::Binary { chunk_count, .. } => *chunk_count,
            StorageType::External { .. } => return Err(Error::UploadNotPending),
        };

        if uploaded_chunks.len() != chunk_count as usize {
            return Err(Error::UploadIncomplete);
        }

        let mut full_data: Vec<u8> = Vec::new();
        CHUNKS.with(|c| {
            let chunks = c.borrow();
            for i in 0..chunk_count {
                if let Some(chunk) = chunks.get(&ChunkKey { sticker_id, chunk_index: i }) {
                    full_data.extend_from_slice(&chunk);
                }
            }
        });
        let dims = parse_image_size(&full_data);

        sticker.upload_status = UploadStatus::Complete;
        if let Some((w, h)) = dims {
            sticker.width = Some(w);
            sticker.height = Some(h);
        }
        sticker.updated_at = ic_cdk::api::time();
        stickers.insert(sticker_id, sticker);

        Ok(())
    })
}

pub fn get_sticker_data(sticker_id: u32) -> Result<Vec<u8>, Error> {
    let sticker = STICKERS.with(|s| {
        s.borrow().get(&sticker_id).ok_or(Error::StickerNotFound)
    })?;

    match &sticker.storage_type {
        StorageType::Binary { chunk_count, .. } => {
            let mut data = Vec::new();
            CHUNKS.with(|c| {
                let chunks = c.borrow();
                for i in 0..*chunk_count {
                    if let Some(chunk) = chunks.get(&ChunkKey { sticker_id, chunk_index: i }) {
                        data.extend_from_slice(&chunk);
                    }
                }
            });
            Ok(data)
        }
        StorageType::External { .. } => {
            Ok(vec![])
        }
    }
}
