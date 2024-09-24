use crate::coding::*;

#[derive(Clone, Debug)]
pub struct Group {
	// The subscribe ID.
	pub subscribe: u64,

	// The group sequence number
	pub sequence: u64,
}

impl Decode for Group {
	fn decode<R: bytes::Buf>(r: &mut R) -> Result<Self, DecodeError> {
		Ok(Self {
			subscribe: u64::decode_more(r, 1)?,
			sequence: u64::decode(r)?,
		})
	}
}

impl Encode for Group {
	fn encode<W: bytes::BufMut>(&self, w: &mut W) {
		self.subscribe.encode(w);
		self.sequence.encode(w);
	}
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GroupOrder {
	#[cfg_attr(feature = "serde", serde(rename = "asc"))]
	Asc,

	#[cfg_attr(feature = "serde", serde(rename = "desc"))]
	Desc,
}

impl Decode for GroupOrder {
	fn decode<R: bytes::Buf>(r: &mut R) -> Result<Self, DecodeError> {
		match u64::decode(r)? {
			0 => Ok(Self::Asc),
			1 => Ok(Self::Desc),
			_ => Err(DecodeError::InvalidValue),
		}
	}
}

impl Encode for GroupOrder {
	fn encode<W: bytes::BufMut>(&self, w: &mut W) {
		let v: u64 = match self {
			Self::Asc => 0,
			Self::Desc => 1,
		};
		v.encode(w)
	}
}

#[derive(Clone, Debug)]
pub struct GroupDrop {
	pub sequence: u64,
	pub count: u64,
	pub code: u32,
}

impl Encode for GroupDrop {
	fn encode<W: bytes::BufMut>(&self, w: &mut W) {
		self.sequence.encode(w);
		self.count.encode(w);
		self.code.encode(w);
	}
}

impl Decode for GroupDrop {
	fn decode<R: bytes::Buf>(r: &mut R) -> Result<Self, DecodeError> {
		Ok(Self {
			sequence: u64::decode_more(r, 2)?,
			count: u64::decode_more(r, 1)?,
			code: u32::decode(r)?,
		})
	}
}
