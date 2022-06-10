use chrono::{DateTime, ParseError};
use futures::future::{self, join_all};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::str::FromStr;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Copy)]
pub enum Level {
    Professional,
    College,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Copy)]
pub enum SportType {
    Hockey,
    Baseball,
    Football(Level),
    Basketball(Level),
    Golf,
}

impl SportType {
    fn to_id(&self) -> u8 {
        match self {
            SportType::Hockey => 0,
            SportType::Baseball => 1,
            SportType::Basketball(Level::College) => 2,
            SportType::Basketball(Level::Professional) => 3,
            SportType::Football(Level::College) => 4,
            SportType::Football(Level::Professional) => 5,
            SportType::Golf => 6,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    FetchError(reqwest::Error),
    ParseError(String),
    SerdeError(serde_json::Error),
    ChronoParseError(ParseError)
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::FetchError(e)
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::SerdeError(e)
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Self::ParseError(s.to_owned())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Self::ParseError(s)
    }
}

impl From<ParseError> for Error {
    fn from(pe: ParseError) -> Self {
        Self::ChronoParseError(pe)
    }
}
#[derive(Debug, Clone, Eq, PartialEq, Default)]
struct Team {
    id: u32,
    location: String,
    name: String,
    display_name: String,
    abbreviation: String,
    primary_color: String,
    secondary_color: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum Status {
    Pregame,
    Active,
    Intermission,
    End,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Game {
    game_id: u64,
    sport_id: SportType,
    home_team: Team,
    away_team: Team,
    home_score: u16,
    away_score: u16,
    status: Status,
    ordinal: String,
    start_time: chrono::DateTime<chrono::Utc>,
    extra: Option<ExtraGameData>
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExtraGameData {
    HockeyData {},
    BaseballData {},
    BasketballData {},
    FootballData {},
    GolfData {},
}

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
    let results = future::join_all(sports.into_iter().map(fetch_sport)).await;
    results.into_iter().collect()
}
async fn fetch_sport(sport: SportType) -> Result<(SportType, Vec<Game>), Error> {
    match sport {
        SportType::Baseball | SportType::Hockey => fetch_statsapi(&sport).await,
        _ => fetch_espn(&sport).await,
    }
    .map(|vec| (sport, vec))
}

async fn fetch_espn(sport: &SportType) -> Result<Vec<Game>, Error> {
    Ok(Vec::new())
}

async fn fetch_statsapi(sport: &SportType) -> Result<Vec<Game>, Error> {
    let (sport_param, suffix) = match sport {
        SportType::Hockey => ("web.nhl", ""),
        SportType::Baseball => ("mlb", "?sportId=1"),
        _ => panic!("Cannot use StatsAPI endpoint for this sport"),
    };
    let schedule_url = format!(
        "http://statsapi.{}.com/api/v1/schedule{}",
        sport_param, suffix
    );

    let resp = reqwest::get(schedule_url).await?.text().await?;
    let json: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&resp)?;
    println!("Got json for sport {:?}", sport);

    let dates = json.get("dates").ok_or("Failed to access dates")?;
    let arr = dates.as_array().ok_or("Dates is not an array")?;

    let mut out_games = Vec::new();
    if let Some(today) = arr.first() {
        let today = today.as_object().ok_or("Today is not an object")?;
        let games = today
            .get("games")
            .ok_or("Missing key games")?
            .as_array()
            .ok_or("Not a list")?;

        for game in games {
            let status = game
                .get("status")
                .ok_or("Could not find status")?
                .as_object()
                .ok_or("Status is not an object")?;
            let detailed_state = status
                .get("detailedState")
                .ok_or("No detailed state present")?;
            if detailed_state == "Postponed" {
                continue;
            } else {
                let game_date = game.get("gameDate").ok_or("No game date present")?.as_str().ok_or("Date is not a string")?;
                println!("Got dame date {game_date}");
                let game_id = game.get("gamePk").ok_or("No game id present")?.as_u64().ok_or("Not an integer")?;

                let g = Game {
                        game_id,
                        sport_id: *sport,
                        home_team: Team::default(),
                        away_team: Team::default(),
                        home_score: 0,
                        away_score: 0,
                        status: Status::Active,
                        ordinal: "".to_owned(),
                        start_time: DateTime::from_str(game_date)?,
                        extra: None
                    };
                out_games.push(g);
            }
        }
    }
    let results = future::join_all(out_games.into_iter().map(fetch_extra)).await;
    results.into_iter().collect()
}

async fn fetch_extra(game: Game) -> Result<Game, Error>{
    match game.sport_id {
        SportType::Baseball => fetch_baseball(game).await,
        SportType::Hockey => fetch_hockey(game).await,
        _ => todo!()
    }
}

async fn fetch_baseball(game: Game) -> Result<Game, Error> {
    println!("Fetching extra data for baseball game {:?}", game.game_id);
    let schedule_url = format!(
        "http://statsapi.mlb.com/api/v1.1/game/{}/feed/live", game.game_id
    );

    let resp = reqwest::get(schedule_url).await?.text().await?;
    let json: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&resp)?;
    println!("Got extra data for baseball game {:?}", game.game_id);
    Ok(game)
}

async fn fetch_hockey(game: Game) -> Result<Game, Error> {
    println!("Fetching extra data for hockey game {:?}", game.game_id);
    let schedule_url = format!(
        "http://statsapi.web.nhl.com/api/v1/game/{}/linescore", game.game_id
    );

    let resp = reqwest::get(schedule_url).await?.text().await?;
    let json: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&resp)?;
    println!("Got extra data for hockey game {:?}", game.game_id);
    Ok(game)
}