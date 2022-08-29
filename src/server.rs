use axum::extract::Path;
use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Extension, Json, Router,
};

use futures::future::join_all;
use live_sports::{fetch_sport, Game, SportType, get_team_map, Team};
use parking_lot::Mutex;
use serde::Deserialize;
use std::net::SocketAddr;
use std::{
    collections::{HashMap},
    sync::Arc,
    time::{Duration, Instant},
};

use itertools::Itertools;

type Cache = HashMap<live_sports::SportType, (Instant, Option<Vec<Game>>)>;

#[tokio::main]
async fn main() {
    // init tracing
    tracing_subscriber::fmt::init();

    let cache = Arc::new(Mutex::new(Cache::new()));

    let app = Router::new()
        .route("/sport/:sport_id", get(get_sport))
        .route("/all", get(get_all))
        .route("/sports", get(get_sports))
        .route("/teams/:sport_id", get(get_teams))
        .layer(Extension(cache));
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Debug, Clone, Deserialize)]
struct SportsRequest {
    sport_ids: Vec<SportType>,
}

async fn get_sports(state: Extension<Arc<Mutex<Cache>>>, Json(request): Json<SportsRequest>) -> impl IntoResponse {
    tracing::info!("Getting sports {:?}", request.sport_ids);
    get_scores_for_sports(state, &request.sport_ids).await.map(|games| {
        Json(games)
    })
}

async fn get_all(state: Extension<Arc<Mutex<Cache>>>) -> impl IntoResponse {
    tracing::info!("Getting all sports");
    get_scores_for_sports(state, &SportType::all_vec()).await.map(|games| {
        Json(games)
    })
}

async fn get_sport(state: Extension<Arc<Mutex<Cache>>>, Path(sport_id): Path<String> ) -> impl IntoResponse {
    tracing::info!("Getting sport data for {}", sport_id);
    let sport = sport_id.parse::<SportType>().map_err(|_| StatusCode::NOT_FOUND)?;
    get_scores_for_sports(state, std::slice::from_ref(&sport)).await.map(|games| {

        games.into_iter().find(|(s_id, _)| *s_id == sport).map(|(_ , g) | {
            Ok(Json(g))
        }).unwrap_or_else(|| {
            Err(StatusCode::NOT_FOUND)
        })
    })
}

async fn get_teams(Path(sport_id): Path<String> ) -> Result<Json<Vec<Team>>, StatusCode> {
    tracing::info!("Getting teams for {}", sport_id);
    let sport = sport_id.parse::<SportType>().map_err(|_| StatusCode::NOT_FOUND)?;
    let team_map = get_team_map(&sport);
    let teams = team_map.into_iter().map(|(_, team)| { team.clone() }).collect_vec();
    Ok(Json(teams))
}



async fn get_scores_for_sports(
    Extension(state): Extension<Arc<Mutex<Cache>>>,
    sports: &[SportType],
) -> Result<HashMap<SportType, Vec<Game>>, StatusCode> {
    let mut results: HashMap<SportType, Vec<Game>> = HashMap::new();
    let mut futures = Vec::new();

    {
        let cache = state.lock();
        for sport in sports {
            if let Some((last_updated, result)) = cache.get(sport) {
                if Instant::now().duration_since(*last_updated) < Duration::from_secs(60) {
                    if let Some(result) = result {
                        results.insert(*sport, result.clone());
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
    Ok(results)
}

#[cfg(test)]
mod test {
    use live_sports::{SportType, Level};

    #[test]
    fn test_request() {
        let request = "{\n    \"sport_ids\": [\n        \"basketball\",\n        \"hockey\"\n    ]\n}";
        let parsed = serde_json::from_str::<super::SportsRequest>(request).unwrap();
        let actual = vec![SportType::Basketball(Level::Professional), SportType::Hockey];
        assert_eq!(parsed.sport_ids, actual);

    }
}