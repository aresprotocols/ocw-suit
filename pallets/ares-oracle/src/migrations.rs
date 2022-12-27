use frame_support::Blake2_128Concat;
use frame_support::storage::types::{OptionQuery, ValueQuery};
use super::*;
use frame_support::traits::{OnRuntimeUpgrade, StorageInstance};
use oracle_finance::types::BalanceOf;

pub struct UpgradeStorageV1<T: crate::Config>(sp_std::marker::PhantomData<T>);
impl<T: crate::Config> OnRuntimeUpgrade for UpgradeStorageV1<T> {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        log::info!(
		    target: "runtime::ares-oracle",
			"update FinalPerCheckResult",
		);

        let write_count: Weight = 1;
        let read_count: Weight = 0;

        <FinalPerCheckResult<T>>::remove_all(None);
        T::DbWeight::get().writes(write_count + read_count)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        Ok(())
    }
}


pub struct PrefixOfPurchasedRequestPool;
impl StorageInstance for PrefixOfPurchasedRequestPool {
    fn pallet_prefix() -> &'static str {
        "OracleFinance"
    }
    const STORAGE_PREFIX: &'static str = "PurchasedRequestPool";
}

pub type OldPurchasedRequestPoolV1<T: crate::Config> = frame_support::storage::types::StorageMap<
    PrefixOfPurchasedRequestPool,
    Blake2_128Concat,
    PurchaseId,
    PurchasedRequestData<T::AccountId, BalanceOf<T, T::FinanceInstance>, T::BlockNumber>,
    OptionQuery,
>;

pub struct PrefixOfPurchasedPricePool;
impl StorageInstance for PrefixOfPurchasedPricePool {
    fn pallet_prefix() -> &'static str {
        "OracleFinance"
    }
    const STORAGE_PREFIX: &'static str = "PurchasedPricePool";
}

pub type OldPurchasedPricePoolV1<T: crate::Config> = frame_support::storage::types::StorageDoubleMap<
    PrefixOfPurchasedPricePool,
    Blake2_128Concat,
    PurchaseId, // purchased_id,
    Blake2_128Concat,
    PriceKey, // price_key,
    PurchasedPriceDataVec<T::AccountId, T::BlockNumber>,
    OptionQuery,
>;

// PurchasedAvgPrice
pub struct PrefixOfPurchasedAvgPrice;
impl StorageInstance for PrefixOfPurchasedAvgPrice {
    fn pallet_prefix() -> &'static str {
        "OracleFinance"
    }
    const STORAGE_PREFIX: &'static str = "PurchasedAvgPrice";
}

pub type OldPurchasedAvgPriceV1 = frame_support::storage::types::StorageDoubleMap<
    PrefixOfPurchasedAvgPrice,
    Blake2_128Concat,
    PurchaseId, // purchased_id,
    Blake2_128Concat,
    PriceKey, // price_key,,
    PurchasedAvgPriceData,
    ValueQuery,
>;

// PurchasedAvgTrace
pub struct PrefixOfPurchasedAvgTrace;
impl StorageInstance for PrefixOfPurchasedAvgTrace {
    fn pallet_prefix() -> &'static str {
        "OracleFinance"
    }
    const STORAGE_PREFIX: &'static str = "PurchasedAvgTrace";
}

pub type OldPurchasedAvgTraceV1<T: crate::Config> = frame_support::storage::types::StorageMap<
    PrefixOfPurchasedAvgTrace,
    Blake2_128Concat,
    PurchaseId, // pricpurchased_ide_key
    T::BlockNumber,
    ValueQuery,
>;

// PurchasedOrderPool
pub struct PrefixOfPurchasedOrderPool;
impl StorageInstance for PrefixOfPurchasedOrderPool {
    fn pallet_prefix() -> &'static str {
        "OracleFinance"
    }
    const STORAGE_PREFIX: &'static str = "PurchasedOrderPool";
}

pub type OldPurchasedOrderPoolV1<T: crate::Config> = frame_support::storage::types::StorageDoubleMap<
    PrefixOfPurchasedOrderPool,
    Blake2_128Concat,
    PurchaseId,
    Blake2_128Concat,
    T::AccountId,
    T::BlockNumber,
    OptionQuery,
>;

pub struct UpdateToV2<T: crate::Config>(sp_std::marker::PhantomData<T>);
impl<T: crate::Config> OnRuntimeUpgrade for UpdateToV2<T> {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {

        if StorageVersion::<T>::get() == Releases::V1_2_0 {
            log::info!(
		    target: "runtime::ares-oracle",
			"Update ares-oracle storage V2",
		);

            OldPurchasedRequestPoolV1::<T>::translate::<PurchasedRequestData<T::AccountId, BalanceOf<T, T::FinanceInstance>, T::BlockNumber>, _>(
                |k, old_value| {
                    log::info!("Write, PurchasedRequestPool::<T>::migration(), {:?}, {:?}", &k, &old_value);
                    PurchasedRequestPool::<T>::insert(OrderIdEnum::String(k), old_value);
                    None
                }
            );

            OldPurchasedPricePoolV1::<T>::translate::<PurchasedPriceDataVec<T::AccountId, T::BlockNumber>, _>(
                |k1, k2, old_value| {
                    log::info!("Write, PurchasedPricePool::<T>::migration(), {:?}, {:?}, {:?}", &k1, &k2, &old_value);
                    PurchasedPricePool::<T>::insert(OrderIdEnum::String(k1), k2, old_value);
                    None
                }
            );

            OldPurchasedAvgPriceV1::translate::<PurchasedAvgPriceData, _>(
                |k1, k2, old_value| {
                    log::info!("Write, PurchasedAvgPrice::<T>::migration(), {:?}, {:?}, {:?}", &k1, &k2, &old_value);
                    PurchasedAvgPrice::<T>::insert(OrderIdEnum::String(k1), k2, old_value);
                    None
                }
            );


            OldPurchasedAvgTraceV1::<T>::translate::<T::BlockNumber, _>(
                |k, old_value| {
                    log::info!("Write, PurchasedAvgTrace::<T>::migration(), {:?}, {:?}", &k, &old_value);
                    PurchasedAvgTrace::<T>::insert(OrderIdEnum::String(k), old_value);
                    None
                }
            );

            OldPurchasedOrderPoolV1::<T>::translate::<T::BlockNumber, _>(
                |k1, k2, old_value| {
                    log::info!("Write, PurchasedOrderPool::<T>::migration(), {:?}, {:?}, {:?}", &k1, &k2, &old_value);
                    PurchasedOrderPool::<T>::insert(OrderIdEnum::String(k1), k2, old_value);
                    None
                }
            );

            StorageVersion::<T>::put(Releases::V2);
        }
        frame_support::weights::Weight::zero()
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        Ok(())
    }
}