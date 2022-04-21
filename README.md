<img src="https://raw.githubusercontent.com/ackwell/ironworks/main/logo.png" alt="ironworks" align="right" height="250">

# ironworks

Modular FFXIV data toolkit written in rust.

---

To minimise unused code & dependencies, ironworks is split into a number of discrete features. No features are enabled by default - pick the ones you want to use!

| Feature  | Description                                                |
| -------- | ---------------------------------------------------------- |
| `excel`  | Read data from Excel databases.                            |
| `ffxiv`  | Bindings for using ironworks with FFXIV.                   |
| `sqpack` | Navigate and extract files from the SqPack package format. |

## Getting started

```toml
[dependencies]
ironworks = {version = "0.1.0", features = ["excel", "ffxiv", "sqpack"]}
```

```rs
use ironworks::{
  Error,
  excel::Excel,
  ffxiv,
  sqpack::SqPack,
};

fn main() -> Result<(), Error> {
  // Read out files directly.
  let sqpack = SqPack::new(ffxiv::FsResource::search().unwrap());
  let file = sqpack.read("exd/root.exl")?;

  // Read fields out of excel.
  let excel = Excel::new(ffxiv::SqpackResource::new(&sqpack));
  let field = excel.sheet("Item")?.row(37362)?.field(0)?;

  Ok(())
}
```