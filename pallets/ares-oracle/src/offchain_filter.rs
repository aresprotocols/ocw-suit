

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
	pub fn is_author_call(extrinsic :&UncheckedExtrinsic<Address, Call, Signature, Extra>) -> bool {
		let in_call = extrinsic.call();
		if let Some(x_call) = Call::try_get_pallet_call(in_call) {
			if let super::pallet::Call::submit_price_unsigned_with_signed_payload {
				price_payload: ref payload,
				signature: ref signature,
			} = x_call {
				let mut block_trace = super::pallet::Pallet::<T>::block_author_trace().unwrap_or(AuthorTraceData::<T>::default());
				let mut block_trace_author = None;
				block_trace.iter().any(|(acc, bn)|{
					if &payload.block_number == bn {
						block_trace_author = Some(acc.clone());
						return true;
					}
					false
				});

				let releation_stash_opt = super::pallet::Pallet::<T>::get_stash_id(&payload.auth);
				if let Some(stash_trace) = block_trace_author {
					if let Some(rel_stash) = releation_stash_opt {
						return &stash_trace == &rel_stash;
					}
				}
				return false
			}
		}

		// if let Call::AresOracle( // Call::AresOracle
		// 	x_call
		// ) = in_call {
		// 	if let super::pallet::Call::submit_price_unsigned_with_signed_payload {
		// 		price_payload: ref payload,
		// 		signature: ref signature,
		// 	} = x_call {
		// 		// let mut block_trace = ares_oracle::pallet::Pallet::<Runtime>::block_author_trace().unwrap_or(AuthorTraceData::<Runtime>::default());
		// 		let mut block_trace = super::pallet::Pallet::<T>::block_author_trace().unwrap_or(AuthorTraceData::<T>::default());
		// 		let mut block_trace_author = None;
		// 		block_trace.iter().any(|(acc, bn)|{
		// 			if &payload.block_number == bn {
		// 				block_trace_author = Some(acc.clone());
		// 				return true;
		// 			}
		// 			false
		// 		});
		//
		// 		let trace_pop1 = block_trace.pop(); //
		// 		let trace_pop2 = block_trace.pop(); //
		// 		let releation_stash_opt = super::pallet::Pallet::<T>::get_stash_id(&payload.auth);
		// 		log::info!("@@@@3 price_payload.stash = {:?} price_payload.block_number =  {:?} block_trace_author = {:?},",
		// 			&releation_stash_opt,
		// 			payload.block_number,
		// 			block_trace_author,
		// 		);
		// 		if let Some(stash_trace) = block_trace_author {
		// 			if let Some(rel_stash) = releation_stash_opt {
		// 				return &stash_trace == &rel_stash;
		// 			}
		// 		}
		// 		return false
		// 	}
		// }

		return true;
	}
}
