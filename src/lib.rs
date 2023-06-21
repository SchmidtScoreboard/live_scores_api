extern crate phf;

pub mod common;
mod sport;

use futures::future::join_all;
use std::collections::{HashMap, HashSet};

pub use common::data::{
    Error, ExtraGameData, Game, GolfPlayer, Level, Possession, SportType, Status,
};

use common::fetch::{fetch_espn, fetch_statsapi};

pub async fn fetch_all() -> Result<HashMap<SportType, Vec<Game>>, Error> {
    let all_sports = {
        let mut s = HashSet::new();
        s.insert(SportType::Hockey);
        s.insert(SportType::Baseball);
        s.insert(SportType::Football(Level::Professional));
        s.insert(SportType::Football(Level::College));
        s.insert(SportType::Basketball(Level::Professional));
        s.insert(SportType::Basketball(Level::College));
        s.insert(SportType::Golf);
        s
    };

    fetch_scores(all_sports).await
}

pub async fn fetch_scores(
    sports: HashSet<SportType>,
) -> Result<HashMap<SportType, Vec<Game>>, Error> {
    let results = join_all(sports.into_iter().map(fetch_sport)).await;
    let mut m = HashMap::new();
    for (sport, result) in results {
        let games = result?;
        m.insert(sport, games);
    }
    Ok(m)
}
pub async fn fetch_sport(sport: SportType) -> (SportType, Result<Vec<Game>, Error>) {
    (
        sport,
        match sport {
            SportType::Hockey => fetch_statsapi(&sport).await,
            _ => fetch_espn(&sport).await,
        },
    )
}
