use super::{Publisher, SessionError, Subscriber};
use crate::{cache::broadcast, setup};
use webtransport_quinn::{RecvStream, SendStream, Session};

/// An endpoint that connects to a URL to publish and/or consume live streams.
pub struct Client {}

impl Client {
	/// Connect using an established WebTransport session, performing the MoQ handshake as a publisher.
	pub async fn publisher(session: Session, source: broadcast::Subscriber) -> Result<Publisher, SessionError> {
		let control = Self::send_setup(&session, setup::Role::Publisher).await?;

		let publisher = Publisher::new(session, control, source);
		Ok(publisher)
	}

	/// Connect using an established WebTransport session, performing the MoQ handshake as a subscriber.
	pub async fn subscriber(session: Session, source: broadcast::Publisher) -> Result<Subscriber, SessionError> {
		let control = Self::send_setup(&session, setup::Role::Subscriber).await?;

		let subscriber = Subscriber::new(session, control, source);
		Ok(subscriber)
	}

	// TODO support performing both roles
	/*
		pub async fn connect(self) -> anyhow::Result<(Publisher, Subscriber)> {
			self.connect_role(setup::Role::Both).await
		}
	*/

	async fn send_setup(session: &Session, role: setup::Role) -> Result<(SendStream, RecvStream), SessionError> {
		let mut control = session.open_bi().await?;

		let client = setup::Client {
			role,
			versions: vec![setup::Version::KIXEL_01].into(),
			params: Default::default(),
		};

		client.encode(&mut control.0).await?;

		let server = setup::Server::decode(&mut control.1).await?;

		if server.version != setup::Version::KIXEL_01 {
			return Err(SessionError::Version(Some(server.version)));
		}

		Ok(control)
	}
}
