use crate::coding::{AsyncRead, AsyncWrite};
use crate::coding::{Decode, DecodeError, Encode, EncodeError, Params};

/// Sent by the subscriber to request all future objects for the given track.
///
/// Objects will use the provided ID instead of the full track name, to save bytes.
#[derive(Clone, Debug)]
pub struct Subscribe {
	/// The subscription ID
	pub id: u64,

	/// Track properties
	pub track_alias: u64, // This alias is useless but part of the spec
	pub track_namespace: String,
	pub track_name: String,

	/// The start/end group/object.
	pub start_group: SubscribeLocation,
	pub start_object: SubscribeLocation,
	pub end_group: SubscribeLocation,
	pub end_object: SubscribeLocation,

	/// Optional parameters
	pub params: Params,
}

impl Subscribe {
	pub async fn decode<R: AsyncRead>(r: &mut R) -> Result<Self, DecodeError> {
		let id = u64::decode(r).await?;
		let track_alias = u64::decode(r).await?;
		let track_namespace = String::decode(r).await?;
		let track_name = String::decode(r).await?;

		let start_group = SubscribeLocation::decode(r).await?;
		let start_object = SubscribeLocation::decode(r).await?;
		let end_group = SubscribeLocation::decode(r).await?;
		let end_object = SubscribeLocation::decode(r).await?;

		// You can't have a start object without a start group.
		if start_group == SubscribeLocation::None && start_object != SubscribeLocation::None {
			return Err(DecodeError::InvalidSubscribeLocation);
		}

		// You can't have an end object without an end group.
		if end_group == SubscribeLocation::None && end_object != SubscribeLocation::None {
			return Err(DecodeError::InvalidSubscribeLocation);
		}

		// NOTE: There's some more location restrictions in the draft, but they're enforced at a higher level.

		let params = Params::decode(r).await?;

		Ok(Self {
			id,
			track_alias,
			track_namespace,
			track_name,
			start_group,
			start_object,
			end_group,
			end_object,
			params,
		})
	}

	pub async fn encode<W: AsyncWrite>(&self, w: &mut W) -> Result<(), EncodeError> {
		self.id.encode(w).await?;
		self.track_alias.encode(w).await?;
		self.track_namespace.encode(w).await?;
		self.track_name.encode(w).await?;

		self.start_group.encode(w).await?;
		self.start_object.encode(w).await?;
		self.end_group.encode(w).await?;
		self.end_object.encode(w).await?;

		self.params.encode(w).await?;

		Ok(())
	}
}

/// Signal where the subscription should begin, relative to the current cache.
#[derive(Clone, Debug, PartialEq)]
pub enum SubscribeLocation {
	None,
	Absolute(u64),
	Latest(u64),
	Future(u64),
}

impl SubscribeLocation {
	pub async fn decode<R: AsyncRead>(r: &mut R) -> Result<Self, DecodeError> {
		let kind = u64::decode(r).await?;

		match kind {
			0 => Ok(Self::None),
			1 => Ok(Self::Absolute(u64::decode(r).await?)),
			2 => Ok(Self::Latest(u64::decode(r).await?)),
			3 => Ok(Self::Future(u64::decode(r).await?)),
			_ => Err(DecodeError::InvalidSubscribeLocation),
		}
	}

	pub async fn encode<W: AsyncWrite>(&self, w: &mut W) -> Result<(), EncodeError> {
		self.id().encode(w).await?;
		match self {
			Self::None => Ok(()),
			Self::Absolute(val) => val.encode(w).await,
			Self::Latest(val) => val.encode(w).await,
			Self::Future(val) => val.encode(w).await,
		}
	}

	fn id(&self) -> u64 {
		match self {
			Self::None => 0,
			Self::Absolute(_) => 1,
			Self::Latest(_) => 2,
			Self::Future(_) => 3,
		}
	}
}
