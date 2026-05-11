use candid::{CandidType, Principal};
use ciborium::{from_reader, into_writer};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum Error {
    NotAdmin,
    NotRegistered,
    NotAuthor,
    UserAlreadyExists,
    UserNotFound,
    PostNotFound,
    CommentNotFound,
    AlreadyLiked,
    NotLiked,
    ImageIndexOutOfRange,
    ChunkIndexOutOfRange,
    ChunkAlreadyUploaded,
    DataTooLarge,
    UploadIncomplete,
    UploadNotPending,
    InvalidContentType,
    ContentTooLong,
    TooManyImages,
}

pub const MAX_CONTENT_LENGTH: usize = 2000;
pub const MAX_IMAGES_PER_POST: u32 = 9;
pub const MAX_CHUNK_SIZE: usize = 1_572_864;
pub const MAX_COMMENT_LENGTH: usize = 500;

pub const ALLOWED_CONTENT_TYPES: &[&str] = &[
    "image/png",
    "image/gif",
    "image/webp",
    "image/jpeg",
];

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct UserProfile {
    pub principal: Principal,
    pub nickname: String,
    pub avatar_url: String,
    pub created_at: u64,
}

impl Storable for UserProfile {
    const BOUND: Bound = Bound::Bounded {
        max_size: 2048,
        is_fixed_size: false,
    };

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        let mut buf = vec![];
        into_writer(self, &mut buf).expect("failed to encode UserProfile");
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        from_reader(&bytes[..]).expect("failed to decode UserProfile")
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum PostStatus {
    Pending { uploaded_images: Vec<u32> },
    Published,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct PostImage {
    pub content_type: String,
    pub size: u64,
    pub chunk_count: u32,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Post {
    pub id: u64,
    pub author: Principal,
    pub content: String,
    pub images: Vec<PostImage>,
    pub status: PostStatus,
    pub like_count: u64,
    pub comment_count: u64,
    pub created_at: u64,
}

impl Storable for Post {
    const BOUND: Bound = Bound::Bounded {
        max_size: 8192,
        is_fixed_size: false,
    };

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        let mut buf = vec![];
        into_writer(self, &mut buf).expect("failed to encode Post");
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        from_reader(&bytes[..]).expect("failed to decode Post")
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct PostSummary {
    pub id: u64,
    pub author: Principal,
    pub content: String,
    pub image_count: u32,
    pub like_count: u64,
    pub comment_count: u64,
    pub created_at: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct PostsPage {
    pub posts: Vec<PostSummary>,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageChunkKey {
    pub post_id: u64,
    pub image_index: u32,
    pub chunk_index: u32,
}

impl Storable for ImageChunkKey {
    const BOUND: Bound = Bound::Bounded {
        max_size: 16,
        is_fixed_size: true,
    };

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        let mut buf = Vec::with_capacity(16);
        buf.extend_from_slice(&self.post_id.to_be_bytes());
        buf.extend_from_slice(&self.image_index.to_be_bytes());
        buf.extend_from_slice(&self.chunk_index.to_be_bytes());
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let b = bytes.as_ref();
        let post_id = u64::from_be_bytes(b[0..8].try_into().unwrap());
        let image_index = u32::from_be_bytes(b[8..12].try_into().unwrap());
        let chunk_index = u32::from_be_bytes(b[12..16].try_into().unwrap());
        Self { post_id, image_index, chunk_index }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LikeKey {
    pub post_id: u64,
    pub user: Principal,
}

impl Storable for LikeKey {
    const BOUND: Bound = Bound::Bounded {
        max_size: 37,
        is_fixed_size: false,
    };

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        let principal_bytes = self.user.as_slice();
        let mut buf = Vec::with_capacity(8 + 1 + principal_bytes.len());
        buf.extend_from_slice(&self.post_id.to_be_bytes());
        buf.push(principal_bytes.len() as u8);
        buf.extend_from_slice(principal_bytes);
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let b = bytes.as_ref();
        let post_id = u64::from_be_bytes(b[0..8].try_into().unwrap());
        let len = b[8] as usize;
        let user = Principal::from_slice(&b[9..9 + len]);
        Self { post_id, user }
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Comment {
    pub id: u64,
    pub post_id: u64,
    pub author: Principal,
    pub content: String,
    pub created_at: u64,
}

impl Storable for Comment {
    const BOUND: Bound = Bound::Bounded {
        max_size: 2048,
        is_fixed_size: false,
    };

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        let mut buf = vec![];
        into_writer(self, &mut buf).expect("failed to encode Comment");
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        from_reader(&bytes[..]).expect("failed to decode Comment")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PostCommentKey {
    pub post_id: u64,
    pub comment_id: u64,
}

impl Storable for PostCommentKey {
    const BOUND: Bound = Bound::Bounded {
        max_size: 16,
        is_fixed_size: true,
    };

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        let mut buf = Vec::with_capacity(16);
        buf.extend_from_slice(&self.post_id.to_be_bytes());
        buf.extend_from_slice(&self.comment_id.to_be_bytes());
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let b = bytes.as_ref();
        let post_id = u64::from_be_bytes(b[0..8].try_into().unwrap());
        let comment_id = u64::from_be_bytes(b[8..16].try_into().unwrap());
        Self { post_id, comment_id }
    }
}
