use crate::storage::ADMINS;
use crate::types::Error;
use candid::Principal;

pub fn is_admin(caller: Principal) -> bool {
    ADMINS.with(|a| a.borrow().contains_key(&caller))
}

pub fn require_admin() -> Result<(), Error> {
    if !is_admin(ic_cdk::caller()) {
        return Err(Error::NotAdmin);
    }
    Ok(())
}

pub fn init_admin(caller: Principal) {
    ADMINS.with(|a| {
        a.borrow_mut().insert(caller, ());
    });
}
