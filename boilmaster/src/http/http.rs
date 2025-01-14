use std::{
	net::{IpAddr, Ipv4Addr, SocketAddr},
	sync::Arc,
};

use anyhow::Result;
use axum::{Extension, Router, Server};
use futures::Future;
use serde::Deserialize;
use tower_http::trace::TraceLayer;

use crate::{data::Data, schema, search::Search};

use super::{search, sheets};

#[derive(Debug, Deserialize)]
pub struct Config {
	address: Option<IpAddr>,
	port: u16,
}

pub async fn serve(
	shutdown: impl Future<Output = ()>,
	config: Config,
	data: Arc<Data>,
	schema: Arc<schema::Provider>,
	search: Arc<Search>,
) -> Result<()> {
	let bind_address = SocketAddr::new(
		config.address.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
		config.port,
	);

	tracing::info!("http binding to {bind_address:?}");

	Server::bind(&bind_address)
		.serve(router(data, schema, search).into_make_service())
		.with_graceful_shutdown(shutdown)
		.await
		.unwrap();

	Ok(())
}

fn router(data: Arc<Data>, schema: Arc<schema::Provider>, search: Arc<Search>) -> Router {
	Router::new()
		.nest("/sheets", sheets::router())
		.nest("/search", search::router(search))
		// TODO: I'm not convinced by setting up the extensions this high, seems a bit magic so to speak
		.layer(Extension(data))
		.layer(Extension(schema))
		.layer(TraceLayer::new_for_http())
}
