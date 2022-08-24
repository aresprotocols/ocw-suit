use codec::Encode;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use crate::{Error, PendingList, StashAccout, WaiterAccout};
use crate::types::{CrossChainInfo, CrossChainInfoList, CrossChainKind, EthereumAddress, Ident};
use bound_vec_helper::BoundVecHelper;

#[test]
fn test_update_waiter() {
	new_test_ext().execute_with(|| {
		let waiter_acc_a: AccountId = 1;
		let waiter_acc_b: AccountId = 2;

		assert_eq!(WaiterAccout::<Test>::get(), None);

		assert_ok!(ManualBridge::update_waiter(Origin::root(), waiter_acc_a));
		System::assert_has_event(Event::ManualBridge(crate::Event::WaiterUpdated { acc: waiter_acc_a }));
		assert_eq!(WaiterAccout::<Test>::get(), Some(waiter_acc_a));

		assert_ok!(ManualBridge::update_waiter(Origin::root(), waiter_acc_b));
		System::assert_has_event(Event::ManualBridge(crate::Event::WaiterUpdated { acc: waiter_acc_b }));
		assert_eq!(WaiterAccout::<Test>::get(), Some(waiter_acc_b));
	});
}

#[test]
fn test_update_stash() {
	new_test_ext().execute_with(|| {
		let stash_acc_a: AccountId = 3;
		let stash_acc_b: AccountId = 4;

		assert_eq!(WaiterAccout::<Test>::get(), None);

		assert_ok!(ManualBridge::update_waiter(Origin::root(), stash_acc_a));
		System::assert_has_event(Event::ManualBridge(crate::Event::WaiterUpdated { acc: stash_acc_a }));
		assert_eq!(WaiterAccout::<Test>::get(), Some(stash_acc_a));

		assert_ok!(ManualBridge::update_waiter(Origin::root(), stash_acc_b));
		System::assert_has_event(Event::ManualBridge(crate::Event::WaiterUpdated { acc: stash_acc_b }));
		assert_eq!(WaiterAccout::<Test>::get(), Some(stash_acc_b));
	});
}

// transfer_to_eth: (eth: T::EthAddress, amount: T::Balance)
#[test]
fn test_transfer_to_eth() {
	new_test_ext().execute_with(|| {
		let stash_acc_a: AccountId = 1;
		let transfer_acc_b: AccountId = 2;
		let eth_addr = EthereumAddress::new([1u8; 20]);

		assert_ok!(ManualBridge::update_minimum_balance_threshold(Origin::root(), 5000));

		// Check balance
		assert_eq!(Balances::free_balance(stash_acc_a), 1000000000100);
		assert_eq!(Balances::free_balance(transfer_acc_b), 2000000000100);

		// If the waiter is not put in service
		assert_eq!(StashAccout::<Test>::get(), None);
		// transfer_to_eth
		assert_noop!(ManualBridge::transfer_to(Origin::signed(transfer_acc_b), CrossChainKind::ETH(eth_addr), 5000), Error::<Test>::StashDoesNotExists);

		// Add new waiter
		assert_ok!(ManualBridge::update_stash(Origin::root(), stash_acc_a));

		// transfer_to_eth but transfer amount is too small
		assert_noop!(ManualBridge::transfer_to(Origin::signed(transfer_acc_b), CrossChainKind::ETH(eth_addr), 1000), Error::<Test>::TransferAmountIsTooSmall);

		assert_ok!(ManualBridge::transfer_to(Origin::signed(transfer_acc_b), CrossChainKind::ETH(eth_addr), 5000));
		assert_eq!(Balances::free_balance(stash_acc_a), 1000000000100 + 5000);
		assert_eq!(Balances::free_balance(transfer_acc_b), 2000000000100 - 5000);

		let current_bn = <frame_system::Pallet<Test>>::block_number();
		let mut new_ident = Ident::create_on_vec(current_bn.encode());
		let _res = new_ident.try_push(0u8);

		System::assert_has_event(Event::ManualBridge(crate::Event::CrossChainRequest {
			acc: transfer_acc_b,
			ident: new_ident,
			kind: CrossChainKind::ETH(eth_addr),
			amount: 5000,
		}));
		
	});
}

#[test]
fn test_set_up_completed_list() {
	new_test_ext().execute_with(|| {
		let waiter_acc_a: AccountId = 1;
		let transfer_acc_b: AccountId = 2;
		let stash_acc_c: AccountId = 3;

		let eth_add_a: EthereumAddress = EthereumAddress::new([1u8; 20]);
		let eth_add_b: EthereumAddress = EthereumAddress::new([2u8; 20]);
		let eth_add_c: EthereumAddress = EthereumAddress::new([3u8; 20]);

		assert_ok!(ManualBridge::update_minimum_balance_threshold(Origin::root(), 5000));

		assert_ok!(ManualBridge::update_waiter(Origin::root(), waiter_acc_a));
		assert_ok!(ManualBridge::update_stash(Origin::root(), stash_acc_c));

		// transfer_to_eth
		assert_ok!(ManualBridge::transfer_to(Origin::signed(transfer_acc_b), CrossChainKind::ETH(eth_add_a), 5000));
		// transfer_to_eth
		assert_ok!(ManualBridge::transfer_to(Origin::signed(transfer_acc_b), CrossChainKind::ETH(eth_add_b), 6000));
		// transfer_to_eth
		assert_ok!(ManualBridge::transfer_to(Origin::signed(transfer_acc_b), CrossChainKind::ETH(eth_add_c), 7000));
		// transfer_to_eth
		assert_ok!(ManualBridge::transfer_to(Origin::signed(transfer_acc_b), CrossChainKind::ETH(eth_add_a), 5000));

		// Get list
		let pend_list = PendingList::<Test>::get(transfer_acc_b).unwrap();
		assert_eq!(pend_list.len(), 4usize);

		let current_bn = <frame_system::Pallet<Test>>::block_number();
		assert_eq!(current_bn, 1 );

		// To complete
		let mut list = CrossChainInfoList::<Test>::default();

		let mut new_ident = Ident::create_on_vec(current_bn.encode());
		let _res = new_ident.try_push(0u8);

		let _res = list.try_push(CrossChainInfo{
			iden: new_ident,
			kind: CrossChainKind::ETH(eth_add_a),
			amount: 5000,
		});

		let mut new_ident = Ident::create_on_vec(current_bn.encode());
		let _res = new_ident.try_push(1u8);

		let _res = list.try_push(CrossChainInfo{
			iden: new_ident,
			kind: CrossChainKind::ETH(eth_add_a), // on error
			amount: 6000,
		});
		//
		assert_noop!(ManualBridge::set_up_completed_list(Origin::signed(waiter_acc_a), transfer_acc_b, list.clone() ), Error::<Test>::CompletedListDataCannotAllMatch);

		// Get list
		let pend_list = PendingList::<Test>::get(transfer_acc_b).unwrap();
		assert_eq!(pend_list.len(), 4usize);
		list.remove(1);

		let mut new_ident = Ident::create_on_vec(current_bn.encode());
		new_ident.try_push(1u8);

		let _res = list.try_push(CrossChainInfo{
			iden: new_ident,
			kind: CrossChainKind::ETH(eth_add_b), // on error
			amount: 6000,
		});

		//
		assert_ok!(ManualBridge::set_up_completed_list(Origin::signed(waiter_acc_a), transfer_acc_b, list.clone() ));
		// Get list
		let pend_list = PendingList::<Test>::get(transfer_acc_b).unwrap();
		assert_eq!(pend_list.len(), 2usize);

		let mut new_ident = Ident::create_on_vec(current_bn.encode());
		let _res = new_ident.try_push(2u8);
		assert_eq!(pend_list[0], CrossChainInfo{
			iden: new_ident,
			kind: CrossChainKind::ETH(eth_add_c),
			amount: 7000
		});

		let mut new_ident = Ident::create_on_vec(current_bn.encode());
		let _res = new_ident.try_push(3u8);
		assert_eq!(pend_list[1], CrossChainInfo{
			iden: new_ident,
			kind: CrossChainKind::ETH(eth_add_a),
			amount: 5000
		});
	});
}