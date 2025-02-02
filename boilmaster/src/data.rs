use std::sync::Arc;

use ironworks::{
	excel::{Excel, Language},
	sqpack::{Install, SqPack},
	Ironworks,
};

pub struct Data {
	// TODO: this should be a lazy map of some kind once this is using real data
	temp_version: Version,
}

impl Data {
	#[allow(clippy::new_without_default)]
	pub fn new() -> Self {
		Data {
			temp_version: Version::new(),
		}
	}

	pub fn version(&self, version: Option<&str>) -> &Version {
		// TODO: actual version handling, pulling data from an actual game install. need patching and all that shit.
		if version.is_some() {
			todo!("data version handling");
		}

		&self.temp_version
	}
}

pub struct Version {
	excel: Arc<Excel<'static>>,
}

impl Version {
	fn new() -> Self {
		// TODO: Work out how to handle languages
		let ironworks = Ironworks::new().with_resource(SqPack::new(Install::search().unwrap()));
		let excel = Excel::with()
			.language(Language::English)
			.build(Arc::new(ironworks));

		Version {
			excel: Arc::new(excel),
		}
	}

	pub fn excel(&self) -> Arc<Excel<'static>> {
		Arc::clone(&self.excel)
	}
}
