pub trait Config: frame_system::Config {
	//copy from pallet_session::Config
	type ValidatorId: sp_runtime::traits::Member
		+ frame_support::Parameter
		+ sp_runtime::traits::MaybeSerializeDeserialize
		+ codec::MaxEncodedLen
		+ sp_std::convert::TryFrom<Self::AccountId>;

	type Keys: sp_runtime::traits::OpaqueKeys
		+ sp_runtime::traits::Member
		+ frame_support::Parameter
		+ sp_runtime::traits::MaybeSerializeDeserialize;

	fn load_keys(v: Self::ValidatorId) -> Option<Self::Keys>;

	fn average_session_length() -> Self::BlockNumber;
}

/*
impl staking_extend::session::Config for Runtime {
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type Keys = network::part_session::SessionKeys;

	fn load_keys(v: Self::ValidatorId) -> Option<Self::Keys> {
		<pallet_session::NextKeys<Self>>::get(v)
	}

	fn average_session_length() -> T::BlockNumber {
		<pallet_session::Pallet<Self>>::average_session_length()
	}
}
*/