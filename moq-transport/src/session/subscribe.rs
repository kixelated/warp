use crate::{
	data, message,
	serve::{self, ServeError, TrackWriter, TrackWriterMode},
	util::State,
};

use super::Subscriber;

struct SubscribeState {
	ok: Option<message::SubscribeOk>,
	closed: Result<(), ServeError>,
}

impl Default for SubscribeState {
	fn default() -> Self {
		Self {
			ok: Default::default(),
			closed: Ok(()),
		}
	}
}

// Held by the application
pub struct Subscribe<S: webtransport_generic::Session> {
	state: State<SubscribeState>,
	subscriber: Subscriber<S>,
	id: u64,
}

impl<S: webtransport_generic::Session> Subscribe<S> {
	pub(super) fn new(mut subscriber: Subscriber<S>, id: u64, track: TrackWriter) -> (Subscribe<S>, SubscribeRecv) {
		subscriber.send_message(message::Subscribe {
			id,
			track_alias: id,
			track_namespace: track.namespace.clone(),
			track_name: track.name.clone(),
			// TODO add these to the publisher.
			start: Default::default(),
			end: Default::default(),
			params: Default::default(),
		});

		let (send, recv) = State::default();

		let send = Subscribe {
			state: send,
			subscriber,
			id,
		};

		let recv = SubscribeRecv {
			state: recv,
			writer: Some(track.into()),
		};

		(send, recv)
	}

	// Block until the subscription is closed.
	pub async fn serve(self) -> Result<(), ServeError> {
		loop {
			let notify = {
				let state = self.state.lock();
				state.closed.clone()?;
				state.modified().ok_or(ServeError::Done)?
			};

			notify.await
		}
	}
}

impl<S: webtransport_generic::Session> Drop for Subscribe<S> {
	fn drop(&mut self) {
		self.subscriber.send_message(message::Unsubscribe { id: self.id });
	}
}

pub(super) struct SubscribeRecv {
	state: State<SubscribeState>,
	writer: Option<TrackWriterMode>,
}

impl SubscribeRecv {
	pub fn recv_ok(&mut self, msg: message::SubscribeOk) -> Result<(), ServeError> {
		let state = self.state.lock();
		if state.ok.is_some() {
			return Err(ServeError::Duplicate);
		}

		let mut state = state.into_mut().ok_or(ServeError::Done)?;
		state.ok = Some(msg);

		Ok(())
	}

	pub fn recv_error(mut self, err: ServeError) -> Result<(), ServeError> {
		let writer = self.writer.take().ok_or(ServeError::Done)?;
		writer.close(err.clone())?;

		let mut state = self.state.lock_mut().ok_or(ServeError::Done)?;
		state.closed = Err(err);

		Ok(())
	}

	pub fn recv_track(&mut self, header: data::TrackHeader) -> Result<serve::StreamWriter, ServeError> {
		let writer = self.writer.take().ok_or(ServeError::Done)?;

		let stream = match writer {
			TrackWriterMode::Track(init) => init.stream(header.send_order)?,
			_ => return Err(ServeError::Mode),
		};

		self.writer = Some(stream.clone().into());

		Ok(stream)
	}

	pub fn recv_group(&mut self, header: data::GroupHeader) -> Result<serve::GroupWriter, ServeError> {
		let writer = self.writer.take().ok_or(ServeError::Done)?;

		let mut groups = match writer {
			TrackWriterMode::Track(init) => init.groups()?,
			TrackWriterMode::Groups(groups) => groups,
			_ => return Err(ServeError::Mode),
		};

		let writer = groups.create(serve::Group {
			group_id: header.group_id,
			priority: header.send_order,
		})?;

		self.writer = Some(groups.into());

		Ok(writer)
	}

	pub fn recv_object(&mut self, header: data::ObjectHeader) -> Result<serve::ObjectWriter, ServeError> {
		let writer = self.writer.take().ok_or(ServeError::Done)?;

		let mut objects = match writer {
			TrackWriterMode::Track(init) => init.objects()?,
			TrackWriterMode::Objects(objects) => objects,
			_ => return Err(ServeError::Mode),
		};

		let writer = objects.create(serve::Object {
			group_id: header.group_id,
			object_id: header.object_id,
			priority: header.send_order,
		})?;

		self.writer = Some(objects.into());

		Ok(writer)
	}

	pub fn recv_datagram(&mut self, datagram: data::Datagram) -> Result<(), ServeError> {
		let writer = self.writer.take().ok_or(ServeError::Done)?;

		let mut datagrams = match writer {
			TrackWriterMode::Track(init) => init.datagrams()?,
			TrackWriterMode::Datagrams(datagrams) => datagrams,
			_ => return Err(ServeError::Mode),
		};

		datagrams.write(serve::Datagram {
			group_id: datagram.group_id,
			object_id: datagram.object_id,
			priority: datagram.send_order,
			payload: datagram.payload,
		})?;

		Ok(())
	}
}
