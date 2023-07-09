use lambda_runtime::{service_fn, LambdaEvent};
use serde::Deserialize;
use std::collections::HashMap;
use tracing::info;

use live_sports::{Game, Sport};

use futures::future::join_all;
use live_sports::fetch_sport;
use live_sports::Error;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use lazy_static::lazy_static; // 1.4.0
use std::str::FromStr;

type Cache = HashMap<String, (Instant, Option<Vec<Game>>)>;

#[derive(Debug, Clone, Deserialize)]
struct SportsRequest {
    sport_ids: Vec<String>,
}

async fn get_sports(request: SportsRequest) -> Result<HashMap<String, Vec<Game>>, Error> {
    tracing::info!("Getting sports {:?}", request.sport_ids);

    let sports: Result<Vec<_>, _> = request
        .sport_ids
        .iter()
        .map(|s| Sport::from_str(s))
        .collect();
    let sports = sports.map_err(|_| Error::InvalidSportType(format!("{:?}", request.sport_ids)))?;
    get_scores_for_sports(&sports).await
}

lazy_static! {
    static ref CACHE: RwLock<Cache> = RwLock::new(Cache::new());
}

async fn get_scores_for_sports(sports: &[Sport]) -> Result<HashMap<String, Vec<Game>>, Error> {
    let mut results: HashMap<String, Vec<Game>> = HashMap::new();
    let mut futures = Vec::new();

    {
        let cache = CACHE
            .read()
            .map_err(|e| Error::InternalError(e.to_string()))?;
        for sport in sports {
            if let Some((last_updated, result)) = cache.get(&sport.to_string()) {
                if Instant::now().duration_since(*last_updated) < Duration::from_secs(60) {
                    if let Some(result) = result {
                        results.insert(sport.to_string(), result.clone());
                    } else {
                        return Err(Error::InternalError("Some weird error".to_string()));
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

    let mut cache = CACHE
        .write()
        .map_err(|e| Error::InternalError(e.to_string()))?;
    for (sport, result) in new_results {
        match result {
            Ok(result) => {
                cache.insert(sport.to_string(), (Instant::now(), Some(result.clone())));
                results.insert(sport.to_string(), result);
            }
            Err(e) => {
                cache.insert(sport.to_string(), (Instant::now(), None));
                tracing::error!("Error when fetching sport {:?}: {:?}", sport, e);
                maybe_err = Some(e);
            }
        }
    }
    if let Some(err) = maybe_err {
        return Err(err);
    }
    Ok(results)
}

async fn func(event: LambdaEvent<SportsRequest>) -> Result<HashMap<String, Vec<Game>>, Error> {
    let (event, _context) = event.into_parts();
    info!("Calling function with event: {:?}", event);
    get_sports(event).await
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    // required to enable CloudWatch error logging by the runtime
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // disable printing the name of the module in every log line.
        .with_target(false)
        // this needs to be set to false, otherwise ANSI color codes will
        // show up in a confusing manner in CloudWatch logs.
        .with_ansi(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    let func = service_fn(func);
    lambda_runtime::run(func).await?;
    Ok(())
}
