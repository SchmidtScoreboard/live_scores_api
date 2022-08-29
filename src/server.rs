use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Extension, Json, Router,
};

use futures::future::join_all;
use live_sports::{fetch_sport, Game, SportType};
use parking_lot::Mutex;
use std::net::SocketAddr;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

type Cache = HashMap<live_sports::SportType, (Instant, Option<Vec<Game>>)>;

#[tokio::main]
async fn main() {
    // init tracing
    tracing_subscriber::fmt::init();

    let cache = Arc::new(Mutex::new(Cache::new()));

    let app = Router::new().layer(Extension(cache)).route("/sport", get(get_scores));
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// async fn get_scores(sports: HashSet<SportType>) -> Result<Json<Vec<Game>>, StatusCode> {

async fn get_scores(state: Extension<Arc<Mutex<Cache>>>) -> impl IntoResponse {
    get_scores_for_set(state, HashSet::new()).await
}

async fn get_scores_for_set(
    Extension(state): Extension<Arc<Mutex<Cache>>>,
    sports: HashSet<SportType>,
) -> impl IntoResponse {
    let mut results: HashMap<SportType, Vec<Game>> = HashMap::new();
    let mut futures = Vec::new();

    {
        let cache = state.lock();
        for sport in sports {
            if let Some((last_updated, result)) = cache.get(&sport) {
                if Instant::now().duration_since(*last_updated) < Duration::from_secs(60) {
                    if let Some(result) = result {
                        results.insert(sport, result.clone());
                    } else {
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                } else {
                    futures.push(fetch_sport(sport));
                }
            } else {
                futures.push(fetch_sport(sport));
            }
        }
    }

    let mut maybe_err = None;
    let new_results = join_all(futures.into_iter()).await;
    let mut cache = state.lock();
    for (sport, result) in new_results {
        match result {
            Ok(result) => {
                cache.insert(sport, (Instant::now(), Some(result.clone())));
                results.insert(sport, result);
            }
            Err(e) => {
                cache.insert(sport, (Instant::now(), None));
                maybe_err = Some(StatusCode::INTERNAL_SERVER_ERROR);
                tracing::error!("Error when fetching sport {:?}: {:?}", sport, e);
            }
        }
    }
    if let Some(err) = maybe_err {
        return Err(err);
    }
    Ok(Json(results))
}
