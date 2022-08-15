#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::BoundedVec;
use frame_support::traits::Get;
use sp_std::convert::TryFrom;
use sp_std::fmt::Debug;
use sp_std::vec::Vec;

pub trait BoundVecHelper<T, S> {
    type Error;
    fn create_on_vec(v: Vec<T>) -> Self;
    fn check_push(&mut self, v: T) ;
    fn try_create_on_vec(v: Vec<T>) -> Result<Self, Self::Error> where Self: Sized;
}

impl <T, S> BoundVecHelper<T, S> for BoundedVec<T, S>
    where
        S: Get<u32>,
        BoundedVec<T, S>: Debug,
        BoundedVec<T, S>: TryFrom<Vec<T>> + Default,
        <BoundedVec<T, S> as TryFrom<Vec<T>>>::Error: Debug,
{
    type Error = <Self as TryFrom<Vec<T>>>::Error;
    fn create_on_vec(v: Vec<T>) -> Self {
        Self::try_from(v).expect("`BoundedVec` MaxEncodedLen Err")
    }
    fn try_create_on_vec(v: Vec<T>) -> Result<Self, Self::Error> where Self: Sized {
        Self::try_from(v)
    }
    fn check_push(&mut self, v: T) {
        self.try_push(v).expect("`BoundedVec` try push err.");
    }
}

#[cfg(test)]
mod tests {
    use frame_support::BoundedVec;
    use frame_support::traits::ConstU32;
    use crate::BoundVecHelper;

    #[test]
    fn test_boundvec_q1 () {
        type DebugLength = ConstU32<3>;
        type DebugBoundVec = BoundedVec<u8, DebugLength>;
        let mut a = DebugBoundVec::default();

        assert_eq!(a.len(), 0);
        a.try_push(b'a');
        a.try_push(b'b');
        a.try_push(b'c');
        assert_eq!(a.len(), 3);
    }

    #[test]
    fn test_ToBoundVec () {
        type TipLength = ConstU32<3>;
        type TestBoundVec = BoundedVec<u8, TipLength> ;

        let origin_v = vec![b'A', b'B'];
        let mut origin_b = TestBoundVec::create_on_vec(origin_v);
        assert_eq!(origin_b.len(), 2);
        origin_b.check_push(b'C');
        assert_eq!(origin_b.len(), 3);
        // println!("--- {:?}", origin_b);
        assert_eq!(origin_b[0], b'A');
        assert_eq!(origin_b[1], b'B');
        assert_eq!(origin_b[2], b'C');
    }
}
