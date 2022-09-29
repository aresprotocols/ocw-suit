use frame_support::pallet_prelude::Hooks;
use sp_runtime::Percent;
use crate::mock::*;

#[test]
fn test_upgrade_v2() {
	let mut t = new_test_ext();
	// t.execute_with(|| {
	//     assert_eq!(AresOcw::conf_pre_check_token_list().len(), 0);
	//     assert_eq!(AresOcw::conf_pre_check_allowable_offset(), Percent::from_percent(0));
	//     assert_eq!(AresOcw::conf_pre_check_session_multi(), 0);
	// });

	t.execute_with(|| {
		// AresOcw::on_runtime_upgrade();
		<AresOcw as Hooks<u64>>::on_runtime_upgrade();
		assert_eq!(AresOcw::conf_pre_check_token_list().len(), 3);
		assert_eq!(AresOcw::conf_pre_check_allowable_offset(), Percent::from_percent(10));
		assert_eq!(AresOcw::conf_pre_check_session_multi(), 2);
	});
}
