

use super::*;
use sp_runtime::generic::UncheckedExtrinsic;
use frame_support::traits::ExtrinsicCall;
use sp_std::fmt::Debug;
use crate::AuthorTraceData;

pub struct AresOracleFilter<T, Address, Call, Signature, Extra>{
	_t: PhantomData<T>,
	_a: PhantomData<Address>,
	_c: PhantomData<Call>,
	_s: PhantomData<Signature>,
	_e: PhantomData<Extra>,
}

impl <T: Config, Address, Call, Signature, Extra> AresOracleFilter<T, Address, Call, Signature, Extra,>
	where T: Config,
		  Extra: sp_runtime::traits::SignedExtension + Debug,
		Address: Debug,
		   Call: Debug + IsAresOracleCall<T, Call>,
		  Signature: Debug,
{
	pub fn is_author_call(extrinsic :&UncheckedExtrinsic<Address, Call, Signature, Extra>, remove_match: bool) -> bool {
		let in_call = extrinsic.call();
		if let Some(x_call) = Call::try_get_pallet_call(in_call) {
			if let super::pallet::Call::submit_price_unsigned_with_signed_payload {
				price_payload: ref payload,
				signature: ref _signature,
			} = x_call {
				let block_trace = super::pallet::Pallet::<T>::block_author_trace().unwrap_or(AuthorTraceData::<T::AccountId, T::BlockNumber>::default());
				let mut block_trace_author = None;
				block_trace.iter().any(|(acc, bn)|{
					if &payload.block_number == bn {
						block_trace_author = Some((acc.clone(), bn.clone()));
						return true;
					}
					false
				});

				let releation_stash_opt = super::pallet::Pallet::<T>::get_stash_id(&payload.auth);
				if let Some(stash_trace) = block_trace_author {
					if let Some(rel_stash) = releation_stash_opt {
						if &stash_trace.0 == &rel_stash {
							if remove_match {
								super::pallet::Pallet::<T>::remove_block_author_on_trace(&stash_trace.0, &stash_trace.1);
							}
							return true;
						}
					}
				}
				return false
			}
		}
		return true;
	}
}
