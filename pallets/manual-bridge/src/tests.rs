use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use crate::{Error, PendingList, WaiterAccout};
use crate::types::{CrossChainInfo, CrossChainInfoList, CrossChainKind, EthereumAddress};

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

// transfer_to_eth: (eth: T::EthAddress, amount: T::Balance)
#[test]
fn test_transfer_to_eth() {
	new_test_ext().execute_with(|| {
		let waiter_acc_a: AccountId = 1;
		let waiter_acc_b: AccountId = 2;
		let eth_addr = EthereumAddress::default();

		// Check balance
		assert_eq!(Balances::free_balance(waiter_acc_a), 1000000000100);
		assert_eq!(Balances::free_balance(waiter_acc_b), 2000000000100);

		// If the waiter is not put in service
		assert_eq!(WaiterAccout::<Test>::get(), None);
		// transfer_to_eth
		assert_noop!(ManualBridge::transfer_to(Origin::signed(waiter_acc_b), CrossChainKind::ETH(eth_addr), 5000), Error::<Test>::WaiterDoesNotExists);

		// Add new waiter
		assert_ok!(ManualBridge::update_waiter(Origin::root(), waiter_acc_a));

		// transfer_to_eth but transfer amount is too small
		assert_noop!(ManualBridge::transfer_to(Origin::signed(waiter_acc_b), CrossChainKind::ETH(eth_addr), 1000), Error::<Test>::TransferAmountIsTooSmall);

		assert_ok!(ManualBridge::transfer_to(Origin::signed(waiter_acc_b), CrossChainKind::ETH(eth_addr), 5000));
		assert_eq!(Balances::free_balance(waiter_acc_a), 1000000000100 + 5000);
		assert_eq!(Balances::free_balance(waiter_acc_b), 2000000000100 - 5000);

		System::assert_has_event(Event::ManualBridge(crate::Event::CrossChainRequest {
			acc: waiter_acc_b,
			kind: CrossChainKind::ETH(eth_addr),
			amount: 5000,
		}));
		
	});
}

#[test]
fn test_set_up_completed_list() {
	new_test_ext().execute_with(|| {
		let waiter_acc_a: AccountId = 1;
		let waiter_acc_b: AccountId = 2;

		let eth_add_a: EthereumAddress = EthereumAddress::new([1u8; 20]);
		let eth_add_b: EthereumAddress = EthereumAddress::new([2u8; 20]);
		let eth_add_c: EthereumAddress = EthereumAddress::new([3u8; 20]);

		assert_ok!(ManualBridge::update_waiter(Origin::root(), waiter_acc_a));

		// transfer_to_eth
		assert_ok!(ManualBridge::transfer_to(Origin::signed(waiter_acc_b), CrossChainKind::ETH(eth_add_a), 5000));
		// transfer_to_eth
		assert_ok!(ManualBridge::transfer_to(Origin::signed(waiter_acc_b), CrossChainKind::ETH(eth_add_b), 6000));
		// transfer_to_eth
		assert_ok!(ManualBridge::transfer_to(Origin::signed(waiter_acc_b), CrossChainKind::ETH(eth_add_c), 7000));
		// transfer_to_eth
		assert_ok!(ManualBridge::transfer_to(Origin::signed(waiter_acc_b), CrossChainKind::ETH(eth_add_a), 5000));

		// Get list
		let pend_list = PendingList::<Test>::get().unwrap();
		assert_eq!(pend_list.len(), 4usize);

		// To complete
		let mut list = CrossChainInfoList::<Test>::default();
		list.try_push(CrossChainInfo{
			acc: waiter_acc_b,
			kind: CrossChainKind::ETH(eth_add_a),
			amount: 5000,
		});
		list.try_push(CrossChainInfo{
			acc: waiter_acc_b,
			kind: CrossChainKind::ETH(eth_add_a), // on error
			amount: 6000,
		});
		//
		assert_noop!(ManualBridge::set_up_completed_list(Origin::signed(waiter_acc_a), list.clone() ), Error::<Test>::CompletedListDataCannotAllMatch);
		// Get list
		let pend_list = PendingList::<Test>::get().unwrap();
		assert_eq!(pend_list.len(), 4usize);
		list.remove(1);
		list.try_push(CrossChainInfo{
			acc: waiter_acc_b,
			kind: CrossChainKind::ETH(eth_add_b), // on error
			amount: 6000,
		});
		//
		assert_ok!(ManualBridge::set_up_completed_list(Origin::signed(waiter_acc_a), list.clone() ));
		// Get list
		let pend_list = PendingList::<Test>::get().unwrap();
		assert_eq!(pend_list.len(), 2usize);
		assert_eq!(pend_list[0], CrossChainInfo{
			acc: waiter_acc_b,
			kind: CrossChainKind::ETH(eth_add_c),
			amount: 7000
		});
		assert_eq!(pend_list[1], CrossChainInfo{
			acc: waiter_acc_b,
			kind: CrossChainKind::ETH(eth_add_a),
			amount: 5000
		});
	});
}