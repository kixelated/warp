use crate::coding::{AsyncRead, AsyncWrite, Decode, DecodeError, Encode, EncodeError};

#[derive(Clone, Debug)]
pub struct GroupHeader {
	// The subscribe ID.
	pub subscribe_id: u64,

	// The track alias.
	pub track_alias: u64,

	// The group sequence number
	pub group_id: u64,

	// The priority, where **smaller** values are sent first.
	pub send_order: u64,
}

impl GroupHeader {
	pub async fn decode<R: AsyncRead>(r: &mut R) -> Result<Self, DecodeError> {
		Ok(Self {
			subscribe_id: u64::decode(r).await?,
			track_alias: u64::decode(r).await?,
			group_id: u64::decode(r).await?,
			send_order: u64::decode(r).await?,
		})
	}

	pub async fn encode<W: AsyncWrite>(&self, w: &mut W) -> Result<(), EncodeError> {
		self.subscribe_id.encode(w).await?;
		self.track_alias.encode(w).await?;
		self.group_id.encode(w).await?;
		self.send_order.encode(w).await?;

		Ok(())
	}
}

#[derive(Clone, Debug)]
pub struct GroupChunk {
	pub object_id: u64,
	pub size: usize,
}

impl GroupChunk {
	pub async fn decode<R: AsyncRead>(r: &mut R) -> Result<Self, DecodeError> {
		Ok(Self {
			object_id: u64::decode(r).await?,
			size: usize::decode(r).await?,
		})
	}

	pub async fn encode<W: AsyncWrite>(&self, w: &mut W) -> Result<(), EncodeError> {
		self.object_id.encode(w).await?;
		self.size.encode(w).await?;

		Ok(())
	}
}
