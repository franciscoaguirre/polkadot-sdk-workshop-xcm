use codec::{Decode, FullCodec, MaxEncodedLen};
use frame_support::{LOG_TARGET, traits::{ProcessMessage, ProcessMessageError}};
use scale_info::TypeInfo;
use sp_std::{fmt::Debug, marker::PhantomData};
use sp_weights::{Weight, WeightMeter};
use xcm::prelude::*;

use crate::xcm_executor::{ExecuteXcm, Outcome};

/// A message processor that delegates execution to an `XcmExecutor`.
pub struct ProcessXcmMessage<MessageOrigin, XcmExecutor, Call>(
	PhantomData<(MessageOrigin, XcmExecutor, Call)>,
);
impl<
		MessageOrigin: Into<Location> + FullCodec + MaxEncodedLen + Clone + Eq + PartialEq + TypeInfo + Debug,
		XcmExecutor: ExecuteXcm<Call>,
		Call,
	> ProcessMessage for ProcessXcmMessage<MessageOrigin, XcmExecutor, Call>
{
	type Origin = MessageOrigin;

	/// Process the given message, using no more than the remaining `weight` to do so.
	fn process_message(
		message: &[u8],
		origin: Self::Origin,
		meter: &mut WeightMeter,
		id: &mut XcmHash,
	) -> Result<bool, ProcessMessageError> {
		let versioned_message = VersionedXcm::<Call>::decode(&mut &message[..]).map_err(|e| {
			log::trace!(
				target: LOG_TARGET,
				"`VersionedXcm` failed to decode: {e:?}",
			);

			ProcessMessageError::Corrupt
		})?;
		let message = Xcm::<Call>::try_from(versioned_message).map_err(|_| {
			log::trace!(
				target: LOG_TARGET,
				"Failed to convert `VersionedXcm` into `XcmV3`.",
			);

			ProcessMessageError::Unsupported
		})?;

		let (consumed, result) = match XcmExecutor::execute(origin.into(), message)
		{
			Outcome::Complete { used } => {
				log::trace!(
					target: LOG_TARGET,
					"XCM message execution complete, used weight: {used}",
				);
				(used, Ok(true))
			},
			Outcome::Incomplete { used, error } => {
				log::trace!(
					target: LOG_TARGET,
					"XCM message execution incomplete, used weight: {used}, error: {error:?}",
				);
				(used, Ok(false))
			},
			// In the error-case we assume the worst case and consume all possible weight.
			Outcome::Error { error } => {
				log::trace!(
					target: LOG_TARGET,
					"XCM message execution error: {error:?}",
				);
				let error = match error {
					xcm::latest::Error::ExceedsStackLimit => ProcessMessageError::Unsupported, // TODO: Use correct error.
					_ => ProcessMessageError::Unsupported,
				};

				(Weight::zero(), Err(error)) // We never weigh messages.
			},
		};
		result
	}
}

