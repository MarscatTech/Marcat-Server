use crate::storage::{IMAGE_CHUNKS, POSTS};
use crate::types::{
    Error, ImageChunkKey, PostImage, PostStatus, ALLOWED_CONTENT_TYPES, MAX_CHUNK_SIZE,
};

pub fn validate_content_type(content_type: &str) -> Result<(), Error> {
    if ALLOWED_CONTENT_TYPES.contains(&content_type) {
        Ok(())
    } else {
        Err(Error::InvalidContentType)
    }
}

pub fn parse_image_size(data: &[u8]) -> Option<(u32, u32)> {
    imagesize::blob_size(data)
        .ok()
        .map(|s| (s.width as u32, s.height as u32))
}

pub fn upload_image_chunk(
    post_id: u64,
    image_index: u32,
    chunk_index: u32,
    data: Vec<u8>,
) -> Result<(), Error> {
    if data.len() > MAX_CHUNK_SIZE {
        return Err(Error::DataTooLarge);
    }

    POSTS.with(|p| {
        let posts = p.borrow();
        let post = posts.get(&post_id).ok_or(Error::PostNotFound)?;

        if image_index as usize >= post.images.len() {
            return Err(Error::ImageIndexOutOfRange);
        }

        let image = &post.images[image_index as usize];
        if chunk_index >= image.chunk_count {
            return Err(Error::ChunkIndexOutOfRange);
        }

        match &post.status {
            PostStatus::Pending { .. } => {}
            PostStatus::Published => return Err(Error::UploadNotPending),
        }

        Ok(())
    })?;

    IMAGE_CHUNKS.with(|c| {
        c.borrow_mut().insert(
            ImageChunkKey {
                post_id,
                image_index,
                chunk_index,
            },
            data,
        );
    });

    Ok(())
}

pub fn finalize_post_images(post_id: u64) -> Result<(), Error> {
    POSTS.with(|p| {
        let mut posts = p.borrow_mut();
        let mut post = posts.get(&post_id).ok_or(Error::PostNotFound)?;

        match &post.status {
            PostStatus::Pending { .. } => {}
            PostStatus::Published => return Err(Error::UploadNotPending),
        }

        for (img_idx, image) in post.images.iter_mut().enumerate() {
            let mut full_data: Vec<u8> = Vec::new();
            IMAGE_CHUNKS.with(|c| {
                let chunks = c.borrow();
                for ci in 0..image.chunk_count {
                    let key = ImageChunkKey {
                        post_id,
                        image_index: img_idx as u32,
                        chunk_index: ci,
                    };
                    if let Some(chunk) = chunks.get(&key) {
                        full_data.extend_from_slice(&chunk)
                    }
                }
            });

            let expected_chunks = image.chunk_count as usize;
            let actual_chunks = IMAGE_CHUNKS.with(|c| {
                let chunks = c.borrow();
                (0..image.chunk_count)
                    .filter(|ci| {
                        chunks.contains_key(&ImageChunkKey {
                            post_id,
                            image_index: img_idx as u32,
                            chunk_index: *ci,
                        })
                    })
                    .count()
            });

            if actual_chunks != expected_chunks {
                return Err(Error::UploadIncomplete);
            }

            if let Some((w, h)) = parse_image_size(&full_data) {
                image.width = Some(w);
                image.height = Some(h);
            }
        }

        post.status = PostStatus::Published;
        posts.insert(post_id, post);
        Ok(())
    })
}

pub fn get_image_data(post_id: u64, image_index: u32) -> Result<(String, Vec<u8>), Error> {
    let post = POSTS.with(|p| p.borrow().get(&post_id).ok_or(Error::PostNotFound))?;

    if image_index as usize >= post.images.len() {
        return Err(Error::ImageIndexOutOfRange);
    }

    let image = &post.images[image_index as usize];
    let content_type = image.content_type.clone();

    let mut data = Vec::new();
    IMAGE_CHUNKS.with(|c| {
        let chunks = c.borrow();
        for ci in 0..image.chunk_count {
            if let Some(chunk) = chunks.get(&ImageChunkKey {
                post_id,
                image_index,
                chunk_index: ci,
            }) {
                data.extend_from_slice(&chunk);
            }
        }
    });

    Ok((content_type, data))
}

pub fn delete_post_images(post_id: u64, images: &[PostImage]) {
    IMAGE_CHUNKS.with(|c| {
        let mut chunks = c.borrow_mut();
        for (img_idx, image) in images.iter().enumerate() {
            for ci in 0..image.chunk_count {
                chunks.remove(&ImageChunkKey {
                    post_id,
                    image_index: img_idx as u32,
                    chunk_index: ci,
                });
            }
        }
    });
}
