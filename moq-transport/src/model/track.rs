use std::{collections::BinaryHeap, ops::Deref, sync::Arc, time};

use indexmap::IndexMap;

use super::{segment, Watch};
use crate::{Error, VarInt};

pub type Track = (Publisher, Subscriber);

pub fn new(name: &str) -> Track {
	let state = Watch::new(State::default());
	let info = Arc::new(Info { name: name.to_string() });

	let publisher = Publisher::new(state.clone(), info.clone());
	let subscriber = Subscriber::new(state, info);

	(publisher, subscriber)
}

#[derive(Debug)]
pub struct Info {
	pub name: String,
}

#[derive(Debug)]
struct State {
	// Store segments in received order so subscribers can detect changes.
	// The key is the segment sequence, which could have gaps.
	// A None value means the segment has expired.
	lookup: IndexMap<VarInt, Option<segment::Subscriber>>,

	// Store when segments will expire in a priority queue.
	expires: BinaryHeap<SegmentExpiration>,

	// The number of None entries removed from the start of the lookup.
	pruned: usize,

	// Set when the publisher is closed/dropped, or all subscribers are dropped.
	closed: Result<(), Error>,
}

impl State {
	pub fn close(&mut self, err: Error) -> Result<(), Error> {
		self.closed?;
		self.closed = Err(err);
		Ok(())
	}

	pub fn insert(&mut self, segment: segment::Subscriber) -> Result<(), Error> {
		self.closed?;

		let entry = match self.lookup.entry(segment.sequence) {
			indexmap::map::Entry::Occupied(_entry) => return Err(Error::Duplicate),
			indexmap::map::Entry::Vacant(entry) => entry,
		};

		if let Some(expires) = segment.expires {
			self.expires.push(SegmentExpiration {
				sequence: segment.sequence,
				expires: time::Instant::now() + expires,
			});
		}

		entry.insert(Some(segment));

		// Expire any existing segments on insert.
		// This means if you don't insert then you won't expire... but it's probably fine since the cache won't grow.
		// TODO Use a timer to expire segments at the correct time instead
		self.expire();

		Ok(())
	}

	// Try expiring any segments
	pub fn expire(&mut self) {
		let now = time::Instant::now();
		while let Some(segment) = self.expires.peek() {
			if segment.expires > now {
				break;
			}

			// Update the entry to None while preserving the index.
			match self.lookup.entry(segment.sequence) {
				indexmap::map::Entry::Occupied(mut entry) => entry.insert(None),
				indexmap::map::Entry::Vacant(_) => panic!("expired segment not found"),
			};

			self.expires.pop();
		}

		// Remove None entries from the start of the lookup.
		while let Some((_, None)) = self.lookup.get_index(0) {
			self.lookup.shift_remove_index(0);
			self.pruned += 1;
		}
	}
}

impl Default for State {
	fn default() -> Self {
		Self {
			lookup: Default::default(),
			expires: Default::default(),
			pruned: 0,
			closed: Ok(()),
		}
	}
}

#[derive(Debug, Clone)]
pub struct Publisher {
	state: Watch<State>,
	info: Arc<Info>,
	_dropped: Arc<Dropped>,
}

impl Publisher {
	fn new(state: Watch<State>, info: Arc<Info>) -> Self {
		let _dropped = Arc::new(Dropped::new(state.clone()));
		Self { state, info, _dropped }
	}

	pub fn insert_segment(&mut self, segment: segment::Subscriber) -> Result<(), Error> {
		self.state.lock_mut().insert(segment)
	}

	// Helper method to create and insert a segment in one step.
	pub fn create_segment(&mut self, info: segment::Info) -> Result<segment::Publisher, Error> {
		let (publisher, subscriber) = segment::new(info);
		self.insert_segment(subscriber)?;
		Ok(publisher)
	}

	pub fn close(self, err: Error) -> Result<(), Error> {
		self.state.lock_mut().close(err)
	}
}

impl Deref for Publisher {
	type Target = Info;

	fn deref(&self) -> &Self::Target {
		&self.info
	}
}

#[derive(Clone, Debug)]
pub struct Subscriber {
	state: Watch<State>,
	info: Arc<Info>,

	// The index of the next segment to return.
	index: usize,

	// If there are multiple segments to return, we put them in here to return them in priority order.
	pending: BinaryHeap<SegmentPriority>,

	// Dropped when all subscribers are dropped.
	_dropped: Arc<Dropped>,
}

impl Subscriber {
	fn new(state: Watch<State>, info: Arc<Info>) -> Self {
		let _dropped = Arc::new(Dropped::new(state.clone()));
		Self {
			state,
			info,
			index: 0,
			pending: Default::default(),
			_dropped,
		}
	}

	pub async fn next_segment(&mut self) -> Result<Option<segment::Subscriber>, Error> {
		loop {
			let notify = {
				let state = self.state.lock();

				// Get our adjusted index, which could be negative if we've removed more broadcasts than read.
				let mut index = self.index.saturating_sub(state.pruned);

				// Push all new segments into a priority queue.
				while index < state.lookup.len() {
					let (_, segment) = state.lookup.get_index(index).unwrap();

					// Skip None values (expired segments).
					// TODO These might actually be expired, so we should check the expiration time.
					if let Some(segment) = segment {
						self.pending.push(SegmentPriority {
							segment: segment.clone(),
						})
					}

					index += 1;
				}

				self.index = state.pruned + index;

				// Return the higher priority segment.
				if let Some(segment) = self.pending.pop() {
					return Ok(Some(segment.segment));
				}

				// Otherwise check if we need to return an error.
				match state.closed {
					Err(Error::Closed) => return Ok(None),
					Err(err) => return Err(err),
					Ok(()) => state.changed(),
				}
			};

			notify.await
		}
	}
}

impl Deref for Subscriber {
	type Target = Info;

	fn deref(&self) -> &Self::Target {
		&self.info
	}
}

// Closes the track on Drop.
#[derive(Debug)]
struct Dropped {
	state: Watch<State>,
}

impl Dropped {
	fn new(state: Watch<State>) -> Self {
		Self { state }
	}
}

impl Drop for Dropped {
	fn drop(&mut self) {
		self.state.lock_mut().close(Error::Closed).ok();
	}
}

// Used to order segments by expiration time.
#[derive(Debug)]
struct SegmentExpiration {
	sequence: VarInt,
	expires: time::Instant,
}

impl Ord for SegmentExpiration {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		// Reverse order so the earliest expiration is at the top of the heap.
		other.expires.cmp(&self.expires)
	}
}

impl PartialOrd for SegmentExpiration {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl PartialEq for SegmentExpiration {
	fn eq(&self, other: &Self) -> bool {
		self.expires == other.expires
	}
}

impl Eq for SegmentExpiration {}

// Used to order segments by priority
#[derive(Debug, Clone)]
struct SegmentPriority {
	segment: segment::Subscriber,
}

impl Ord for SegmentPriority {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		// Reverse order so the highest priority is at the top of the heap.
		// TODO I let CodePilot generate this code so yolo
		other.segment.priority.cmp(&self.segment.priority)
	}
}

impl PartialOrd for SegmentPriority {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl PartialEq for SegmentPriority {
	fn eq(&self, other: &Self) -> bool {
		self.segment.priority == other.segment.priority
	}
}

impl Eq for SegmentPriority {}
