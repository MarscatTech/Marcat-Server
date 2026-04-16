use crate::types::{ChunkKey, PackStickerKey, Sticker, StickerPack};
use candid::Principal;
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    DefaultMemoryImpl, StableBTreeMap, StableCell,
};
use std::cell::RefCell;

type Memory = VirtualMemory<DefaultMemoryImpl>;

const PACKS_MEM_ID: MemoryId = MemoryId::new(0);
const STICKERS_MEM_ID: MemoryId = MemoryId::new(1);
const CHUNKS_MEM_ID: MemoryId = MemoryId::new(2);
const PACK_STICKERS_MEM_ID: MemoryId = MemoryId::new(3);
const NEXT_PACK_ID_MEM_ID: MemoryId = MemoryId::new(4);
const NEXT_STICKER_ID_MEM_ID: MemoryId = MemoryId::new(5);
const ADMINS_MEM_ID: MemoryId = MemoryId::new(6);

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    pub static PACKS: RefCell<StableBTreeMap<u64, StickerPack, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(PACKS_MEM_ID)),
        ));

    pub static STICKERS: RefCell<StableBTreeMap<u64, Sticker, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(STICKERS_MEM_ID)),
        ));

    pub static CHUNKS: RefCell<StableBTreeMap<ChunkKey, Vec<u8>, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(CHUNKS_MEM_ID)),
        ));

    pub static PACK_STICKERS: RefCell<StableBTreeMap<PackStickerKey, (), Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(PACK_STICKERS_MEM_ID)),
        ));

    pub static NEXT_PACK_ID: RefCell<StableCell<u64, Memory>> =
        RefCell::new(StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(NEXT_PACK_ID_MEM_ID)),
            1_u64,
        ).unwrap());

    pub static NEXT_STICKER_ID: RefCell<StableCell<u64, Memory>> =
        RefCell::new(StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(NEXT_STICKER_ID_MEM_ID)),
            1_u64,
        ).unwrap());

    pub static ADMINS: RefCell<StableBTreeMap<Principal, (), Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(ADMINS_MEM_ID)),
        ));
}

pub fn next_pack_id() -> u64 {
    NEXT_PACK_ID.with(|cell| {
        let id = *cell.borrow().get();
        cell.borrow_mut().set(id + 1).unwrap();
        id
    })
}

pub fn next_sticker_id() -> u64 {
    NEXT_STICKER_ID.with(|cell| {
        let id = *cell.borrow().get();
        cell.borrow_mut().set(id + 1).unwrap();
        id
    })
}
