use crate::coding::*;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Control {
	Session,
	Announce,
	Subscribe,
	Datagrams,
	Fetch,
	Info,
}

impl Decode for Control {
	fn decode<R: bytes::Buf>(r: &mut R) -> Result<Self, DecodeError> {
		let t = u64::decode(r)?;
		match t {
			0 => Ok(Self::Session),
			1 => Ok(Self::Announce),
			2 => Ok(Self::Subscribe),
			3 => Ok(Self::Datagrams),
			4 => Ok(Self::Fetch),
			5 => Ok(Self::Info),
			_ => Err(DecodeError::InvalidValue),
		}
	}
}

impl Encode for Control {
	fn encode<W: bytes::BufMut>(&self, w: &mut W) -> Result<(), EncodeError> {
		let v: u64 = match self {
			Self::Session => 0,
			Self::Announce => 1,
			Self::Subscribe => 2,
			Self::Datagrams => 3,
			Self::Fetch => 4,
			Self::Info => 5,
		};
		v.encode(w)
	}
}
