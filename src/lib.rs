pub mod common;
mod sport;

use futures::future::join_all;
use std::collections::{HashMap, HashSet};

pub use common::data::Error;

pub use common::proto_helpers::{all_sports, new_sport};
pub use common::team::get_team_map;
pub use common::types::sport::{Level, SportType};
pub use common::types::{Game, Sport};

use common::fetch::{fetch_espn, fetch_statsapi};

pub async fn fetch_all() -> Result<HashMap<Sport, Vec<Game>>, Error> {
    fetch_scores(all_sports().into_iter().collect()).await
}

pub async fn fetch_scores(sports: HashSet<Sport>) -> Result<HashMap<Sport, Vec<Game>>, Error> {
    let results = join_all(sports.into_iter().map(fetch_sport)).await;
    let mut m = HashMap::new();
    for (sport, result) in results {
        let games = result?;
        m.insert(sport, games);
    }
    Ok(m)
}
pub async fn fetch_sport(sport: Sport) -> (Sport, Result<Vec<Game>, Error>) {
    (
        sport,
        match sport.sport_type() {
            SportType::Hockey => fetch_statsapi(&sport).await,
            _ => fetch_espn(&sport).await,
        },
    )
}
