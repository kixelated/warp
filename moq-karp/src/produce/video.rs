use moq_transfork::coding::*;

use crate::media::Timestamp;

pub struct Video {
	inner: moq_transfork::TrackProducer,
	group: Option<moq_transfork::GroupProducer>,
}

impl Video {
	pub(super) fn new(inner: moq_transfork::TrackProducer) -> Self {
		Self { inner, group: None }
	}

	pub fn keyframe(&mut self) {
		// The take() is important, it means we'll create a new group on the next write.
		if let Some(group) = self.group.take() {
			tracing::debug!(sequence = group.sequence, frames = group.frame_count(), "keyframe");
		}
	}

	pub fn write<B: Into<Bytes>>(&mut self, timestamp: Timestamp, payload: B) {
		let timestamp = timestamp.as_micros();
		let mut header = BytesMut::with_capacity(timestamp.encode_size());
		timestamp.encode(&mut header);

		let mut group = match self.group.take() {
			Some(group) => group,
			None => self.inner.append_group(),
		};

		let payload = payload.into();
		let mut frame = group.create_frame(header.len() + payload.len());
		frame.write(header.freeze());
		frame.write(payload);

		self.group.replace(group);
	}
}
