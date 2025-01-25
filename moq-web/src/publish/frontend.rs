use url::Url;
use wasm_bindgen::prelude::*;
use web_sys::MediaStream;

use super::{Backend, Controls, ControlsSend, Status, StatusRecv};
use crate::{Error, Result};

#[wasm_bindgen]
pub struct Publish {
	controls: ControlsSend,
	status: StatusRecv,
}

#[wasm_bindgen]
impl Publish {
	#[wasm_bindgen(constructor)]
	pub fn new() -> Self {
		let controls = Controls::default().baton();
		let status = Status::default().baton();

		let backend = Backend::new(controls.1, status.0);
		backend.start();

		Self {
			controls: controls.0,
			status: status.1,
		}
	}

	#[wasm_bindgen(getter)]
	pub fn url(&self) -> Option<String> {
		self.controls.url.get().map(|u| u.to_string())
	}

	#[wasm_bindgen(setter)]
	pub fn set_url(&mut self, url: Option<String>) -> Result<()> {
		let url = match url {
			Some(url) => Url::parse(&url).map_err(|_| Error::InvalidUrl(url.to_string()))?.into(),
			None => None,
		};
		self.controls.url.set(url);
		Ok(())
	}

	#[wasm_bindgen(getter)]
	pub fn media(&self) -> Option<MediaStream> {
		self.controls.media.get()
	}

	#[wasm_bindgen(setter)]
	pub fn set_media(&mut self, media: Option<MediaStream>) {
		self.controls.media.set(media)
	}

	#[wasm_bindgen(getter)]
	pub fn volume(&self) -> f64 {
		self.controls.volume.get()
	}

	#[wasm_bindgen(setter)]
	pub fn set_volume(&mut self, volume: f64) {
		self.controls.volume.set(volume);
	}

	#[wasm_bindgen(getter)]
	pub fn error(&self) -> Option<String> {
		self.status.error.get().map(|e| e.to_string())
	}

	pub fn states(&self) -> PublishStates {
		PublishStates {
			state: self.status.state.clone(),
		}
	}
}

impl Default for Publish {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug, Default, Copy, Clone)]
#[wasm_bindgen]
pub enum PublishState {
	#[default]
	Idle,
	Connecting,
	Connected,
	Live,
	Error,
}

// Unfortunately, we need this wrapper because `wasm_bindgen` doesn't support generics.
#[wasm_bindgen]
pub struct PublishStates {
	state: baton::Recv<PublishState>,
}

#[wasm_bindgen]
impl PublishStates {
	pub async fn next(&mut self) -> Option<PublishState> {
		self.state.next().await
	}
}
