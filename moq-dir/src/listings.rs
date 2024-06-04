use std::{
	collections::HashMap,
	sync::{Arc, Mutex},
};

use moq_transfork::serve::{Broadcast, BroadcastReader, BroadcastWriter, ServeError};

use crate::{ListingReader, ListingWriter};

struct State {
	writer: BroadcastWriter,
	active: HashMap<String, ListingWriter>,
}

#[derive(Clone)]
pub struct Listings {
	state: Arc<Mutex<State>>,
	broadcast: BroadcastReader,
}

impl Listings {
	pub fn new(broadcast: String) -> Self {
		let (writer, reader) = Broadcast::new(broadcast).produce();

		let state = State {
			writer,
			active: HashMap::new(),
		};

		Self {
			state: Arc::new(Mutex::new(state)),
			broadcast: reader,
		}
	}

	// Returns a Registration that removes on drop.
	pub fn register(&mut self, path: &str) -> Result<Option<Registration>, ServeError> {
		let (prefix, base) = Self::prefix(path);

		if !prefix.starts_with(&self.broadcast.name) {
			// Ignore anything that isn't in our broadcast.
			return Ok(None);
		}

		// Remove the broadcast prefix from the path.
		let prefix = &prefix[self.broadcast.name.len()..];

		let mut state = self.state.lock().unwrap();
		if let Some(listing) = state.active.get_mut(prefix) {
			listing.insert(base.to_string())?;
		} else {
			log::info!("creating prefix: {}", prefix);
			let track = state.writer.create_track(prefix).build().unwrap();

			let mut listing = ListingWriter::new(track);
			listing.insert(base.to_string())?;
			state.active.insert(prefix.to_string(), listing);
		}

		log::info!("added listing: {} {}", prefix, base);

		Ok(Some(Registration {
			listing: self.clone(),
			prefix: prefix.to_string(),
			base: base.to_string(),
		}))
	}

	fn remove(&mut self, prefix: &str, base: &str) -> Result<(), ServeError> {
		let mut state = self.state.lock().unwrap();

		let listing = state.active.get_mut(prefix).ok_or(ServeError::NotFound)?;
		listing.remove(base)?;

		log::info!("removed listing: {} {}", prefix, base);

		if listing.is_empty() {
			log::info!("removed prefix: {}", prefix);
			state.active.remove(prefix);
			state.writer.remove_track(prefix);
		}

		Ok(())
	}

	pub fn subscribe(&mut self, name: &str) -> Option<ListingReader> {
		self.broadcast.get_track(name).map(ListingReader::new)
	}

	pub fn broadcast(&self) -> BroadcastReader {
		self.broadcast.clone()
	}

	// Returns the prefix for the string.
	// This is just the content before the last '/', like a directory name.
	// ex. "/foo/bar/baz" -> ("/foo/bar", "baz")
	pub fn prefix(path: &str) -> (&str, &str) {
		// Find the last '/' and return the parts.
		match path.rfind('.') {
			Some(index) => (&path[..index + 1], &path[index + 1..]),
			None => ("", path),
		}
	}
}

// Used to remove the registration on drop.
pub struct Registration {
	listing: Listings,
	prefix: String,
	base: String,
}

impl Drop for Registration {
	fn drop(&mut self) {
		self.listing.remove(&self.prefix, &self.base).ok();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_bucket() {
		assert!(Listings::prefix(".") == (".", ""));
		assert!(Listings::prefix(".foo") == (".", "foo"));
		assert!(Listings::prefix(".foo.") == (".foo.", ""));
		assert!(Listings::prefix(".foo.bar") == (".foo.", "bar"));
		assert!(Listings::prefix(".foo.bar.") == (".foo.bar.", ""));
		assert!(Listings::prefix(".foo.bar.baz") == (".foo.bar.", "baz"));
		assert!(Listings::prefix(".foo.bar.baz.") == (".foo.bar.baz.", ""));

		assert!(Listings::prefix("") == ("", ""));
		assert!(Listings::prefix("foo") == ("", "foo"));
		assert!(Listings::prefix("foo.") == ("foo.", ""));
		assert!(Listings::prefix("foo.bar") == ("foo.", "bar"));
		assert!(Listings::prefix("foo.bar.") == ("foo.bar.", ""));
		assert!(Listings::prefix("foo.bar.baz") == ("foo.bar.", "baz"));
		assert!(Listings::prefix("foo.bar.baz.") == ("foo.bar.baz.", ""));
	}
}
