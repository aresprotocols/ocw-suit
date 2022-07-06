use super::*;

impl<T: Config> Pallet<T> {

    /// Get the `ares-authority` through `stash-id`
    pub fn get_auth_id(stash: &T::AccountId) -> Option<T::AuthorityAres> {
        let authority_list = <Authorities<T>>::get();
        if(authority_list.is_none()) {
            return None;
        }
        for (storage_stash, auth) in authority_list.unwrap().into_iter() {
            if stash == &storage_stash {
                return Some(auth);
            }
        }
        None
    }

    /// Get the `stash-id` through `ares-authority`
    pub fn get_stash_id(auth: &T::AuthorityAres) -> Option<T::AccountId> {
        let authorities =  <Authorities<T>>::get() ;
        if authorities.is_none() {
            return None;
        }
        let authorities = authorities.unwrap();
        for (stash, storage_auth) in authorities.into_iter() {
            if auth == &storage_auth {
                return Some(stash);
            }
        }
        None
    }

    /// Get all `ares-authorities` users in keystore.
    pub fn get_ares_authority_list() -> Vec<T::AuthorityAres> {
        let authority_list = T::AuthorityAres::all(); // T::AuthorityAres::all();
        authority_list
    }

    /// Check whether the authority of the current block author has a private key on the local node.
    pub fn check_block_author_and_sotre_key_the_same(block_author: &T::AuthorityAres) -> bool {
        let mut is_same = !<OcwControlSetting<T>>::get().need_verifier_check;
        if is_same {
            log::warn!(
				target: "pallet::are_block_author_and_sotre_key_the_same",
				"❗❗❗ verifier_check is disable, current status is debug."
			);
            return true;
        }
        let worker_ownerid_list = Self::get_ares_authority_list();
        worker_ownerid_list.iter().any(|local_auth| {
            log::debug!("check_block_author_and_sotre_key_the_same . Local: {:?}, BlockAuthor: {:?}", &local_auth, &block_author);
            local_auth == block_author
        })
    }

}

