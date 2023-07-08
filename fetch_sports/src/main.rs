use axum::{extract::Path, http::StatusCode, response::Json, routing::get, Extension, Router};
use lambda_http::{run, Error};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

use live_sports::{Game, Sport};

use futures::future::join_all;
use live_sports::all_sports;
use live_sports::common::types::Team;
use live_sports::{common::team::get_team_map, fetch_sport};
use parking_lot::Mutex;
use std::time::{Duration, Instant};

use itertools::Itertools;

type Cache = HashMap<String, (Instant, Option<Vec<Game>>)>;

#[derive(Debug, Clone, Deserialize)]
struct SportsRequest {
    sport_ids: Vec<Sport>,
}

async fn get_sports(
    state: Extension<Arc<Mutex<Cache>>>,
    Json(request): Json<SportsRequest>,
) -> Result<Json<HashMap<String, Vec<Game>>>, StatusCode> {
    tracing::info!("Getting sports {:?}", request.sport_ids);
    let scores = get_scores_for_sports(state, &request.sport_ids).await?;
    Ok(Json(scores))
}

async fn get_all(
    state: Extension<Arc<Mutex<Cache>>>,
) -> Result<Json<HashMap<String, Vec<Game>>>, StatusCode> {
    tracing::info!("Getting all sports");
    let scores = get_scores_for_sports(state, &all_sports()).await?;
    tracing::info!("Got all sports");
    Ok(Json(scores))
}

async fn get_sport(
    state: Extension<Arc<Mutex<Cache>>>,
    Path(sport_id): Path<String>,
) -> Result<Json<Vec<Game>>, StatusCode> {
    tracing::info!("Getting sport data for {}", sport_id);
    let sport = sport_id
        .parse::<Sport>()
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let games = get_scores_for_sports(state, &[sport]).await?;
    let sport_string = sport.to_string();
    let (_, games) = games
        .into_iter()
        .find(|(s_id, _)| *s_id == sport_string)
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(games))
}

async fn get_teams(Path(sport_id): Path<String>) -> Result<Json<Vec<Team>>, StatusCode> {
    tracing::info!("Getting teams for {}", sport_id);
    let sport = sport_id
        .parse::<Sport>()
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let team_map = get_team_map(&sport);
    let teams = team_map.iter().map(|(_, team)| team.clone()).collect_vec();
    Ok(Json(teams))
}

async fn get_scores_for_sports(
    Extension(state): Extension<Arc<Mutex<Cache>>>,
    sports: &[Sport],
) -> Result<HashMap<String, Vec<Game>>, StatusCode> {
    let mut results: HashMap<String, Vec<Game>> = HashMap::new();
    let mut futures = Vec::new();

    {
        let cache = state.lock();
        for sport in sports {
            if let Some((last_updated, result)) = cache.get(&sport.to_string()) {
                if Instant::now().duration_since(*last_updated) < Duration::from_secs(60) {
                    if let Some(result) = result {
                        results.insert(sport.to_string(), result.clone());
                    } else {
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                } else {
                    futures.push(fetch_sport(*sport));
                }
            } else {
                futures.push(fetch_sport(*sport));
            }
        }
    }

    let mut maybe_err = None;
    let new_results = join_all(futures.into_iter()).await;
    let mut cache = state.lock();
    for (sport, result) in new_results {
        match result {
            Ok(result) => {
                cache.insert(sport.to_string(), (Instant::now(), Some(result.clone())));
                results.insert(sport.to_string(), result);
            }
            Err(e) => {
                cache.insert(sport.to_string(), (Instant::now(), None));
                maybe_err = Some(StatusCode::INTERNAL_SERVER_ERROR);
                tracing::error!("Error when fetching sport {:?}: {:?}", sport, e);
            }
        }
    }
    if let Some(err) = maybe_err {
        return Err(err);
    }
    Ok(results)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // required to enable CloudWatch error logging by the runtime
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        // disable printing the name of the module in every log line.
        .with_target(false)
        // this needs to be set to false, otherwise ANSI color codes will
        // show up in a confusing manner in CloudWatch logs.
        .with_ansi(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    info!("Starting app");

    let cache = Arc::new(Mutex::new(Cache::new()));
    let app = Router::new()
        .route("/sport/:sport_id", get(get_sport))
        .route("/all", get(get_all))
        .route("/sports", get(get_sports))
        .route("/teams/:sport_id", get(get_teams))
        .layer(Extension(cache));

    run(app).await
}
