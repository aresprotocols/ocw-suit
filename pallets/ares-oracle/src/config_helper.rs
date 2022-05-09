use super::*;

impl<T: Config> Pallet<T> {

    /// Get the maximum depth of the `price` pool.
    pub fn get_price_pool_depth() -> u32 {
        <PricePoolDepth<T>>::get()
    }

}

