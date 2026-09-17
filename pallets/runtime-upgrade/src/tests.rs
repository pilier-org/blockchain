use crate::{Event, mock::*};
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;
use sp_runtime::DispatchError;

#[test]
fn authorize_upgrade_by_origin_succeeds() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let code_hash = H256::repeat_byte(0xAB);
        assert_ok!(RuntimeUpgrade::authorize_upgrade(
            RuntimeOrigin::root(),
            code_hash
        ));

        // `frame_system` records the authorization exactly as its own `authorize_upgrade`
        // extrinsic would (with version checking on, though `CodeUpgradeAuthorization` only
        // exposes its `code_hash` publicly — `check_version` has no accessor outside
        // `frame_system` itself).
        let authorization = System::authorized_upgrade().expect("upgrade should be authorized");
        assert_eq!(authorization.code_hash(), &code_hash);

        System::assert_last_event(Event::UpgradeAuthorized { code_hash }.into());
    });
}

#[test]
fn authorize_upgrade_rejects_non_origin() {
    new_test_ext().execute_with(|| {
        let code_hash = H256::repeat_byte(0xAB);

        assert_noop!(
            RuntimeUpgrade::authorize_upgrade(RuntimeOrigin::signed(1), code_hash),
            DispatchError::BadOrigin
        );

        assert!(System::authorized_upgrade().is_none());
    });
}
