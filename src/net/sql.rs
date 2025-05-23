use super::headers::Accept;
use super::AppState;
use crate::cnf::HTTP_MAX_SQL_BODY_SIZE;
use crate::err::Error;
use crate::net::input::bytes_to_utf8;
use crate::net::output;
use crate::net::params::Params;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::extract::DefaultBodyLimit;
use axum::extract::Query;
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use axum::routing::options;
use axum::Extension;
use axum::Router;
use axum_extra::TypedHeader;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use surrealdb::dbs::capabilities::RouteTarget;
use surrealdb::dbs::Session;
use tower_http::limit::RequestBodyLimitLayer;

pub(super) fn router<S>() -> Router<S>
where
	S: Clone + Send + Sync + 'static,
{
	Router::new()
		.route("/sql", options(|| async {}).get(get_handler).post(post_handler))
		.route_layer(DefaultBodyLimit::disable())
		.layer(RequestBodyLimitLayer::new(*HTTP_MAX_SQL_BODY_SIZE))
}

async fn post_handler(
	Extension(state): Extension<AppState>,
	Extension(session): Extension<Session>,
	output: Option<TypedHeader<Accept>>,
	params: Query<Params>,
	sql: Bytes,
) -> Result<impl IntoResponse, impl IntoResponse> {
	// Get a database reference
	let db = &state.datastore;
	// Check if capabilities allow querying the requested HTTP route
	if !db.allows_http_route(&RouteTarget::Sql) {
		warn!("Capabilities denied HTTP route request attempt, target: '{}'", &RouteTarget::Sql);
		return Err(Error::ForbiddenRoute(RouteTarget::Sql.to_string()));
	}
	// Check if the user is allowed to query
	if !db.allows_query_by_subject(session.au.as_ref()) {
		return Err(Error::ForbiddenRoute(RouteTarget::Sql.to_string()));
	}
	// Convert the received sql query
	let sql = bytes_to_utf8(&sql)?;
	// Execute the received sql query
	match db.execute(sql, &session, params.0.parse().into()).await {
		Ok(res) => match output.as_deref() {
			// Simple serialization
			Some(Accept::ApplicationJson) => Ok(output::json(&output::simplify(res)?)),
			Some(Accept::ApplicationCbor) => Ok(output::cbor(&output::simplify(res)?)),
			// Internal serialization
			Some(Accept::Surrealdb) => Ok(output::full(&res)),
			// An incorrect content-type was requested
			_ => Err(Error::InvalidType),
		},
		// There was an error when executing the query
		Err(err) => Err(Error::from(err)),
	}
}

async fn get_handler(
	ws: WebSocketUpgrade,
	Extension(state): Extension<AppState>,
	Extension(sess): Extension<Session>,
) -> impl IntoResponse {
	ws.on_upgrade(move |socket| handle_socket(state, socket, sess))
}

async fn handle_socket(state: AppState, ws: WebSocket, session: Session) {
	// Split the WebSocket connection
	let (mut tx, mut rx) = ws.split();
	// Wait to receive the next message
	while let Some(res) = rx.next().await {
		if let Ok(msg) = res {
			if let Ok(sql) = msg.to_text() {
				// Get a database reference
				let db = &state.datastore;
				// Execute the received sql query
				let _ = match db.execute(sql, &session, None).await {
					// Convert the response to JSON
					Ok(v) => match serde_json::to_string(&v) {
						// Send the JSON response to the client
						Ok(v) => tx.send(Message::Text(v)).await,
						// There was an error converting to JSON
						Err(e) => tx.send(Message::Text(Error::from(e).to_string())).await,
					},
					// There was an error when executing the query
					Err(e) => tx.send(Message::Text(Error::from(e).to_string())).await,
				};
			}
		}
	}
}
