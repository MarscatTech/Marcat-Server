use candid::{CandidType, Principal};
use ciborium::{from_reader, into_writer};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct StickerInfo {
    pub id: u32,
    pub pack_id: u32,
    pub name: String,
    pub storage_type: StorageType,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct PackSummary {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub cover_sticker_id: Option<u64>,
    pub creator: Principal,
    pub created_at: u64,
    pub updated_at: u64,
    pub sticker_count: u32,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct PacksPage {
    pub packs: Vec<PackSummary>,
    pub total: u32,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct StickersPage {
    pub stickers: Vec<StickerInfo>,
    pub total: u32,
}

pub const MAX_CHUNK_SIZE: usize = 1_572_864;

pub const MAX_ZIP_SIZE: usize = 10 * 1024 * 1024;

pub const MAX_ZIP_IMAGES: usize = 100;

pub const ALLOWED_CONTENT_TYPES: &[&str] = &[
    "image/png",
    "image/gif",
    "image/webp",
    "image/jpeg",
];

pub fn extension_to_content_type(ext: &str) -> Option<&'static str> {
    match ext.to_lowercase().as_str() {
        "png" => Some("image/png"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        _ => None,
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum Error {
    NotAdmin,
    PackNotFound,
    StickerNotFound,
    UploadNotPending,
    ChunkIndexOutOfRange,
    ChunkAlreadyUploaded,
    DataTooLarge,
    UploadIncomplete,
    InvalidContentType,
    ZipTooLarge,
    ZipTooManyImages,
    ZipNoImagesFound,
    ZipExtractFailed { detail: String },
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum StorageType {
    Binary {
        content_type: String,
        size: u64,
        chunk_count: u32,
    },
    External {
        url: String,
    },
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum UploadStatus {
    Complete,
    Pending { uploaded_chunks: Vec<u32> },
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct StickerPack {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub cover_sticker_id: Option<u64>,
    pub creator: Principal,
    pub created_at: u64,
    pub updated_at: u64,
}

impl Storable for StickerPack {
    const BOUND: Bound = Bound::Bounded {
        max_size: 2048,
        is_fixed_size: false,
    };

    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buf = vec![];
        into_writer(self, &mut buf).expect("failed to encode StickerPack");
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        from_reader(&bytes[..]).expect("failed to decode StickerPack")
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Sticker {
    pub id: u32,
    pub pack_id: u32,
    pub name: String,
    pub tags: Vec<String>,
    pub storage_type: StorageType,
    pub upload_status: UploadStatus,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl Storable for Sticker {
    const BOUND: Bound = Bound::Bounded {
        max_size: 2048,
        is_fixed_size: false,
    };

    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buf = vec![];
        into_writer(self, &mut buf).expect("failed to encode Sticker");
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        from_reader(&bytes[..]).expect("failed to decode Sticker")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChunkKey {
    pub sticker_id: u32,
    pub chunk_index: u32,
}

impl Storable for ChunkKey {
    const BOUND: Bound = Bound::Bounded {
        max_size: 8,
        is_fixed_size: true,
    };

    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buf = Vec::with_capacity(8);
        buf.extend_from_slice(&self.sticker_id.to_be_bytes());
        buf.extend_from_slice(&self.chunk_index.to_be_bytes());
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let b = bytes.as_ref();
        let sticker_id = u32::from_be_bytes(b[0..4].try_into().unwrap());
        let chunk_index = u32::from_be_bytes(b[4..8].try_into().unwrap());
        Self { sticker_id, chunk_index }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackStickerKey {
    pub pack_id: u32,
    pub sticker_id: u32,
}

impl Storable for PackStickerKey {
    const BOUND: Bound = Bound::Bounded {
        max_size: 8,
        is_fixed_size: true,
    };

    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buf = Vec::with_capacity(8);
        buf.extend_from_slice(&self.pack_id.to_be_bytes());
        buf.extend_from_slice(&self.sticker_id.to_be_bytes());
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let b = bytes.as_ref();
        let pack_id = u32::from_be_bytes(b[0..4].try_into().unwrap());
        let sticker_id = u32::from_be_bytes(b[4..8].try_into().unwrap());
        Self { pack_id, sticker_id }
    }
}
