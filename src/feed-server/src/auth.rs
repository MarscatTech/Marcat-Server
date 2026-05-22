use crate::storage::{ADMINS, USERS};
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

pub fn is_registered(caller: Principal) -> bool {
    USERS.with(|u| u.borrow().contains_key(&caller))
}

pub fn require_registered() -> Result<(), Error> {
    if !is_registered(ic_cdk::caller()) {
        return Err(Error::NotRegistered);
    }
    Ok(())
}

pub fn init_admin(caller: Principal) {
    ADMINS.with(|a| {
        a.borrow_mut().insert(caller, ());
    });
}
