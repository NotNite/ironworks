use std::{
	cell::RefCell,
	collections::{hash_map::Entry, HashMap},
	path::PathBuf,
	rc::Rc,
};

use crate::{
	dat_reader::DatReader,
	error::{Error, Result},
};

/// Representation of a top-level repository directory within a SqPack database.
#[derive(Debug)]
pub struct Repository {
	/// Name of the repository as it appears in SqPack file paths.
	pub name: String,
	/// Numeric ID of the repository as used in SqPack archive file names.
	pub id: u8,
	/// File path to the location of the repository directory on disk.
	pub path: PathBuf,
}

/// Representation of a data category within a SqPack database.
#[derive(Debug)]
pub struct Category {
	/// Name of the category as it appears in SqPack file paths.
	pub name: String,
	/// Numeric ID of the category as used in SqPack archive file names.
	pub id: u8,
}

/// Representation of a group of SqPack archive files that form a single database.
#[derive(Debug)]
pub struct SqPack {
	default_repository: String,
	repositories: HashMap<String, Rc<Repository>>,
	categories: HashMap<String, Rc<Category>>,

	reader_cache: RefCell<HashMap<(String, String), Rc<DatReader>>>,
}

impl SqPack {
	// TODO: Should we sanity check the default repo at this point?
	/// Build a representation of a SqPack database. It is expected that the
	/// `default_repository` is a valid `name` for a provided repository.
	pub fn new(
		default_repository: String,
		repositories: impl IntoIterator<Item = Repository>,
		categories: impl IntoIterator<Item = Category>,
	) -> Self {
		SqPack {
			default_repository,

			repositories: repositories
				.into_iter()
				.map(|repository| (repository.name.to_owned(), Rc::new(repository)))
				.collect(),

			categories: categories
				.into_iter()
				.map(|category| (category.name.to_owned(), Rc::new(category)))
				.collect(),

			reader_cache: RefCell::new(HashMap::new()),
		}
	}

	/// Try to read the file at `sqpack_path` as raw bytes from the SqPack database.
	pub fn read_file(&self, sqpack_path: &str) -> Result<Vec<u8>> {
		let sqpack_path = sqpack_path.to_lowercase();
		let reader = self.get_reader(&sqpack_path)?;
		reader.read_file(&sqpack_path)
	}

	fn get_reader(&self, sqpack_path: &str) -> Result<Rc<DatReader>> {
		// TODO: maybe try_borrow_mut?
		let mut cache = self.reader_cache.borrow_mut();

		// Check if we have a reader for the given metadata, and return it if we do.
		let (category_name, repository_name) = self.parse_segments(sqpack_path)?;
		let vacant_entry = match cache.entry((category_name.into(), repository_name.into())) {
			Entry::Occupied(entry) => return Ok(entry.get().clone()),
			Entry::Vacant(entry) => entry,
		};

		// No existing reader found, build a new one and store in the cache.
		let repository = self.get_repository(repository_name)?;
		let category = self.get_category(category_name)?;
		let reader = Rc::new(DatReader::new(repository, category)?);

		Ok(vacant_entry.insert(reader).clone())
	}

	fn parse_segments<'b>(&self, path: &'b str) -> Result<(&'b str, &'b str)> {
		// TODO: consider itertools or similar if we find this pattern a few times
		let split = path.splitn(3, '/').take(2).collect::<Vec<_>>();
		match split[..] {
			[category_name, repository_name] => Ok((category_name, repository_name)),
			_ => Err(Error::InvalidPath(path.to_owned())),
		}
	}

	fn get_repository(&self, repository_name: &str) -> Result<Rc<Repository>> {
		self.repositories
			.get(repository_name)
			.or_else(|| self.repositories.get(&self.default_repository))
			.cloned()
			.ok_or_else(|| Error::UnknownPathSegment {
				segment_type: String::from("repository"),
				segment: repository_name.to_owned(),
			})
	}

	fn get_category(&self, category_name: &str) -> Result<Rc<Category>> {
		self.categories
			.get(category_name)
			.cloned()
			.ok_or_else(|| Error::UnknownPathSegment {
				segment_type: String::from("category"),
				segment: category_name.to_owned(),
			})
	}
}