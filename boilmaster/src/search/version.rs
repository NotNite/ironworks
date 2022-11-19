use std::{
	collections::HashMap,
	path::PathBuf,
	sync::{Arc, RwLock},
};

use anyhow::Result;
use futures::{stream::FuturesUnordered, StreamExt};

use crate::data::Version as DataVersion;

use super::index::Index;

pub struct Version {
	path: PathBuf,

	indices: RwLock<HashMap<String, Index>>,
}

impl Version {
	pub(super) fn new(path: PathBuf) -> Self {
		Self {
			path,
			indices: Default::default(),
		}
	}

	pub(super) async fn ingest(self: Arc<Self>, data: &DataVersion) -> Result<()> {
		let excel = data.excel();

		// NOTE: should probably record which sheets contain strings so we can immediately ignore the rest when there's a query string

		// TODO: on zipatch-backed data instances, accessing .list() could block for quite some time - how do i want to handle that?
		// Create a group of futures; one for each sheet that (should) exist in the index - indexes will be ingested if they do not yet exist.
		let list = excel.list()?;
		let mut futures = list
			.iter()
			.map(|sheet_name| {
				let excel = excel.clone();
				let this = self.clone();

				async move {
					let sheet = match excel.sheet(sheet_name.to_string()) {
						Ok(v) => v,
						Err(err) => anyhow::bail!(err),
					};

					let index =
						Index::ingest(this.path.join(sheet_name.replace('/', "!DIR!")), sheet)
							.await?;
					Ok((sheet_name, index))
				}
			})
			.collect::<FuturesUnordered<_>>();

		// Pull in all the futures as they complete, adding to the index map.
		while let Some(result) = futures.next().await {
			// TODO: Error handling - a failure here probably implies a failed ingestion, which is Not Good.
			let (sheet_name, index) = result.expect("Ingestion failure, this is bad.");
			self.indices
				.write()
				.unwrap()
				.insert(sheet_name.to_string(), index);
		}

		// TODO: should we have some atomic bool in the version to mark when the full task is complete - we probably don't want partial completion.

		Ok(())
	}

	// TODO: index specifier?
	// TODO: non-string-query filters
	// TODO: continuation?
	// TODO: Nicer type (that struct would be real handy around about now)
	#[allow(clippy::type_complexity)]
	pub fn search(&self, query: &str) -> Result<Vec<(f32, (String, u32, u16))>> {
		let indices = self.indices.read().expect("TODO error poisoned");

		// Get an iterator for each of the indexes, lifting any errors from the initial search execution.
		let index_results = indices
			.iter()
			.map(|(name, index)| {
				let tagged_results = index
					.search(query)?
					.map(|(score, (row, subrow))| (score, (name.to_owned(), row, subrow)));
				Ok(tagged_results)
			})
			.collect::<Result<Vec<_>>>()?;

		// TODO: this just groups by index, effectively - should probably sort by score at this point
		// Merge the results from each index into a single vector.
		let results = index_results.into_iter().flatten().collect::<Vec<_>>();

		Ok(results)
	}
}
