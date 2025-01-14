use std::{cell::RefCell, collections::HashMap};

use moq_karp::moq_transfork::{self, web_transport};
use tokio::sync::watch;
use url::Url;
use wasm_bindgen_futures::spawn_local;

use crate::{Error, Result};

// Can't use LazyLock in WASM because nothing is Sync
thread_local! {
	static CACHE: RefCell<HashMap<Url, Session>> = RefCell::new(HashMap::new());
}

#[derive(Clone)]
pub struct Session {
	connected: watch::Receiver<Option<Result<moq_transfork::Session>>>,
}

impl Session {
	pub fn new(addr: Url) -> Self {
		// Use a global cache to share sessions between elements.
		CACHE.with(|cache| {
			let mut cache = cache.borrow_mut();

			let entry = cache.entry(addr.clone()).or_insert_with(|| Self::create(addr.clone()));
			entry.clone()
		})
	}

	fn create(addr: Url) -> Self {
		let (tx, rx) = watch::channel(None);

		// Use a background task to make `connect` cancel safe.
		spawn_local(async move {
			let session = Self::run(&addr).await;
			tx.send(Some(session.clone())).ok();

			if let Ok(session) = session {
				tokio::select! {
					// Close the session gracefully when there are no more consumers.
					_ = tx.closed() => session.close(moq_transfork::Error::Cancel),

					// Remove the session from the cache when it's closed.
					err = session.closed() => {
						tracing::warn!(?err, "session closed");
						CACHE.with(|cache| {
							cache.borrow_mut().remove(&addr);
						});
					},
				}
			}
		});

		Self { connected: rx }
	}

	async fn run(addr: &Url) -> Result<moq_transfork::Session> {
		let client = web_transport::Client::new().congestion_control(web_transport::CongestionControl::LowLatency);

		let session = match addr.scheme() {
			"http" => {
				// TODO Unfortunately, WebTransport doesn't work correctly with self-signed certificates.
				// Until that gets fixed, we need to perform a HTTP request to fetch the certificate hashes.
				let fingerprint = Self::fingerprint(&addr).await?;
				let client = client.server_certificate_hashes(vec![fingerprint]);

				// Make a copy of the address, changing it from HTTP to HTTPS for WebTransport:
				let mut addr = addr.clone();
				let _ = addr.set_scheme("https");
				client.connect(&addr).await?
			}
			"https" => client.connect(&addr).await?,
			_ => return Err(Error::InvalidUrl),
		};

		let session = moq_transfork::Session::connect(session).await?;
		Ok(session)
	}

	async fn fingerprint(url: &Url) -> Result<Vec<u8>> {
		let mut fingerprint = url.clone();
		fingerprint.set_path("fingerprint");

		let resp = gloo_net::http::Request::get(fingerprint.as_str()).send().await?;

		let body = resp.text().await?;
		let fingerprint = hex::decode(body.trim()).map_err(|_| Error::InvalidFingerprint)?;

		Ok(fingerprint)
	}

	pub async fn connect(&mut self) -> Result<moq_transfork::Session> {
		self.connected
			.wait_for(Option::is_some)
			.await
			.expect("background task panicked")
			.as_ref()
			.unwrap()
			.clone()
	}
}
