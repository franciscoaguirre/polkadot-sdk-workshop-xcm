use frame_support::{assert_ok, traits::{Everything, fungible::Inspect}};
use fundamentals_pallet_xcm::Pallet as PalletXcm;
use xcm::{latest::prelude::*, VersionedAssets, VersionedLocation, VersionedXcm};
use xcm_builder::{
	AllowUnpaidExecutionFrom, ConvertedConcreteId, EnsureXcmOrigin, FrameTransactionalProcessor,
	FungibleAdapter, IsConcrete, NoChecking, NonFungiblesAdapter,
};
use xcm_executor::traits::JustTry;
use xcm_simulator::TestExt;

use crate::constants::ALICE;
use crate::network::{ParaA, ParaB, MockNet};
use crate::network::parachain::{
	self,
	UniversalLocation, ParentLocation, LocationConverter, AccountId,
	Balances, LocalOriginToLocation, RuntimeOrigin, XcmRouter, Runtime,
};
use crate::{
	pallet_xcm::pallet as fundamentals_pallet_xcm, xcm_executor::*,
};

#[test]
fn execute_works() {
	ParaA::execute_with(|| {
		// Alice and bob might have some non-zero starting balance.
		let alice_original_balance = Balances::balance(&ALICE);
		const BOB: sp_runtime::AccountId32 = sp_runtime::AccountId32::new([2u8; 32]);
		let bob_location: Location = AccountId32 { id: BOB.into(), network: None }.into();
		let bob_original_balance = Balances::balance(&BOB);

		let alice_origin: RuntimeOrigin = frame_system::RawOrigin::Signed(ALICE).into();

		let asset: Asset = (Parent, 100u128).into();
		let message = Xcm::<parachain::RuntimeCall>::builder_unsafe()
			.transfer_asset(asset, bob_location)
			.build();
		let versioned_message = Box::new(VersionedXcm::V4(message));
		assert_ok!(PalletXcm::<Runtime>::execute(alice_origin, versioned_message, Weight::default()));

		// Alice's balance is updated
		assert_eq!(Balances::balance(&ALICE), alice_original_balance - 100u128);
		assert_eq!(Balances::balance(&BOB), bob_original_balance + 100u128);
	});
}

#[test]
fn do_teleport_works() {
	//sp_tracing::init_for_tests();
	MockNet::reset();
	pub const BOB: sp_runtime::AccountId32 = sp_runtime::AccountId32::new([2u8; 32]);

	let bob_bytes: [u8; 32] = BOB.into();
	let mut bob_original_balance = 0;

	ParaB::execute_with(|| {
		// Alice and bob might have some non-zero starting balance.
		bob_original_balance = Balances::balance(&BOB);
	});

	ParaA::execute_with(|| {
		// Alice and bob might have some non-zero starting balance.
		let alice_original_balance = Balances::balance(&ALICE);

		let alice_origin: RuntimeOrigin = frame_system::RawOrigin::Signed(ALICE).into();

		let dest: Location = Location::new(1, [Parachain(2)]);
		let bob_account: Location = Location::new(0, [AccountId32 { id: bob_bytes.into(), network: None }]);
		let asset: Asset = (Parent, 100u128).into();

		let v_dest = Box::new(VersionedLocation::V4(dest));
		let v_bob_dest = Box::new(VersionedLocation::V4(bob_account));
		let v_asset = Box::new(VersionedAssets::V4(asset.into()));

		assert_ok!(PalletXcm::<Runtime>::teleport_assets(
			alice_origin,
			v_dest,
			v_bob_dest,
			v_asset,
			0
		));

		// Alice's balance is updated
		assert_eq!(Balances::balance(&ALICE), alice_original_balance - 100u128);
	});

	ParaB::execute_with(|| {
		// Bob's balance is updated
		assert_eq!(Balances::balance(&BOB), bob_original_balance + 100u128);
	});
}
