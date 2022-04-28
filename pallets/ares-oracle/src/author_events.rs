use crate::{AuthorTraceData, BlockAuthor, BlockAuthorTrace, Config, Pallet};
use frame_system::{ self as system };

impl<T: Config + pallet_authorship::Config>
pallet_authorship::EventHandler<T::AccountId, T::BlockNumber> for Pallet<T>
{
    fn note_author(author: T::AccountId) {
        BlockAuthor::<T>::put(author.clone());
        let current_block_num: T::BlockNumber = <system::Pallet<T>>::block_number();
        if let Some(mut old_trace) = BlockAuthorTrace::<T>::get() {
            if old_trace.len() > 5 {
                old_trace.remove(0u32 as usize);
            }
            old_trace.try_push((author,current_block_num));
            BlockAuthorTrace::<T>::put(old_trace);
        }else{
            let mut trace_data = AuthorTraceData::<T>::default();
            trace_data.try_push((author,current_block_num));
            BlockAuthorTrace::<T>::put(trace_data);
        }
    }

    fn note_uncle(_author: T::AccountId, _age: T::BlockNumber) { }
}