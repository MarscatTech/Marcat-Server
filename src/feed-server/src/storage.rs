use crate::types::{
    Comment, CommentReplyKey, FollowKey, ImageChunkKey, LikeKey, Post, PostCommentKey, UserProfile,
};
use candid::Principal;
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    DefaultMemoryImpl, StableBTreeMap, StableCell,
};
use std::cell::RefCell;

type Memory = VirtualMemory<DefaultMemoryImpl>;

const USERS_MEM_ID: MemoryId = MemoryId::new(0);
const POSTS_MEM_ID: MemoryId = MemoryId::new(1);
const IMAGE_CHUNKS_MEM_ID: MemoryId = MemoryId::new(2);
const LIKES_MEM_ID: MemoryId = MemoryId::new(3);
const COMMENTS_MEM_ID: MemoryId = MemoryId::new(4);
const POST_COMMENTS_MEM_ID: MemoryId = MemoryId::new(5);
const NEXT_POST_ID_MEM_ID: MemoryId = MemoryId::new(6);
const NEXT_COMMENT_ID_MEM_ID: MemoryId = MemoryId::new(7);
const ADMINS_MEM_ID: MemoryId = MemoryId::new(8);
const COMMENT_REPLIES_MEM_ID: MemoryId = MemoryId::new(9);
const FOLLOWS_MEM_ID: MemoryId = MemoryId::new(10);

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    pub static USERS: RefCell<StableBTreeMap<Principal, UserProfile, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(USERS_MEM_ID)),
        ));

    pub static POSTS: RefCell<StableBTreeMap<u64, Post, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(POSTS_MEM_ID)),
        ));

    pub static IMAGE_CHUNKS: RefCell<StableBTreeMap<ImageChunkKey, Vec<u8>, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(IMAGE_CHUNKS_MEM_ID)),
        ));

    pub static LIKES: RefCell<StableBTreeMap<LikeKey, (), Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(LIKES_MEM_ID)),
        ));

    pub static COMMENTS: RefCell<StableBTreeMap<u64, Comment, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(COMMENTS_MEM_ID)),
        ));

    pub static POST_COMMENTS: RefCell<StableBTreeMap<PostCommentKey, (), Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(POST_COMMENTS_MEM_ID)),
        ));

    pub static NEXT_POST_ID: RefCell<StableCell<u64, Memory>> =
        RefCell::new(StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(NEXT_POST_ID_MEM_ID)),
            1_u64,
        ).unwrap());

    pub static NEXT_COMMENT_ID: RefCell<StableCell<u64, Memory>> =
        RefCell::new(StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(NEXT_COMMENT_ID_MEM_ID)),
            1_u64,
        ).unwrap());

    pub static ADMINS: RefCell<StableBTreeMap<Principal, (), Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(ADMINS_MEM_ID)),
        ));

    pub static COMMENT_REPLIES: RefCell<StableBTreeMap<CommentReplyKey, (), Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(COMMENT_REPLIES_MEM_ID)),
        ));

    pub static FOLLOWS: RefCell<StableBTreeMap<FollowKey, (), Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(FOLLOWS_MEM_ID)),
        ));
}

pub fn next_post_id() -> u64 {
    NEXT_POST_ID.with(|cell| {
        let id = *cell.borrow().get();
        cell.borrow_mut().set(id + 1).unwrap();
        id
    })
}

pub fn next_comment_id() -> u64 {
    NEXT_COMMENT_ID.with(|cell| {
        let id = *cell.borrow().get();
        cell.borrow_mut().set(id + 1).unwrap();
        id
    })
}
