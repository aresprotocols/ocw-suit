use super::*;

//For Ares Call
pub struct EnsureProportionNoMoreThan<N: U32, D: U32, AccountId, I: 'static>(
	sp_std::marker::PhantomData<(N, D, AccountId, I)>,
);
impl<O: Into<Result<RawOrigin<AccountId, I>, O>> + From<RawOrigin<AccountId, I>>, N: U32, D: U32, AccountId, I>
	EnsureOrigin<O> for EnsureProportionNoMoreThan<N, D, AccountId, I>
{
	type Success = ();
	fn try_origin(o: O) -> Result<Self::Success, O> {
		o.into().and_then(|o| match o {
			RawOrigin::Members(n, m) if n * D::VALUE <= N::VALUE * m => Ok(()),
			r => Err(O::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn successful_origin() -> O {
		O::from(RawOrigin::Members(1u32, 0u32))
	}
}