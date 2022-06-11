extern crate phf;

mod team;

use chrono::{DateTime, ParseError};
use futures::future::join_all;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use serde_json::{Map, Value};
use team::Team;

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

#[derive(Debug, Clone, Eq, PartialEq)]
enum Status {
    Pregame,
    Active,
    Intermission,
    End,
    Invalid,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Game {
    game_id: u64,
    sport_id: SportType,
    home_team: Option<Team>,
    away_team: Option<Team>,
    home_score: u64,
    away_score: u64,
    status: Status,
    ordinal: String,
    start_time: chrono::DateTime<chrono::Utc>,
    extra: Option<ExtraGameData>
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExtraGameData {
    HockeyData {
        away_powerplay: bool,
        home_powerplay: bool,
        away_players: u64,
        home_players: u64
    },
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
    let results = join_all(sports.into_iter().map(fetch_sport)).await;
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
fn get_object_from_value<'a>(object: &'a Value, name: &'static str) -> Result<&'a Map<String, Value>, Error> {
    Ok(object.get(name).ok_or(format!("{name} not present {object}"))?.as_object().ok_or(format!("{name} is not an object"))?)
}
fn get_array_from_value<'a>(object: &'a Value, name: &'static str) -> Result<&'a Vec<Value>, Error> {
    Ok(object.get(name).ok_or(format!("{name} not present {object}"))?.as_array().ok_or(format!("{name} is not an array"))?)
}

fn get_object<'a>(object: &'a Map<String, Value>, name: &'static str) -> Result<&'a Map<String, Value>, Error> {
    Ok(object.get(name).ok_or(format!("{name} not present {object:?}"))?.as_object().ok_or(format!("{name} is not an object"))?)
}
fn get_array<'a>(object: &'a Map<String, Value>, name: &'static str) -> Result<&'a Vec<Value>, Error> {
    Ok(object.get(name).ok_or(format!("{name} not present {object:?}"))?.as_array().ok_or(format!("{name} is not an array"))?)
}
fn get_str<'a>(object: &'a Map<String, Value>, name: &'static str) -> Result<&'a str, Error> {
    Ok(object.get(name).ok_or(format!("{name} not present {object:?}"))?.as_str().ok_or(format!("{name} is not a string"))?)
}
fn get_u64(object: &Map<String, Value>, name: &'static str) -> Result<u64, Error> {
    Ok(object.get(name).ok_or(format!("{name} not present {object:?}"))?.as_u64().ok_or(format!("{name} is not an integer"))?)
}
fn get_bool(object: &Map<String, Value>, name: &'static str) -> Result<bool, Error> {
    Ok(object.get(name).ok_or(format!("{name} not present {object:?}"))?.as_bool().ok_or(format!("{name} is not a boolean"))?)
}

async fn fetch_statsapi(sport: &SportType) -> Result<Vec<Game>, Error> {
    let (sport_param, suffix, team_map) = match sport {
        SportType::Hockey => ("web.nhl", "", &team::HOCKEY_TEAMS),
        SportType::Baseball => ("mlb", "?sportId=1", &team::BASEBALL_TEAMS),
        _ => panic!("Cannot use StatsAPI endpoint for this sport"),
    };
    let schedule_url = format!(
        "http://statsapi.{}.com/api/v1/schedule{}",
        sport_param, suffix
    );

    let resp = reqwest::get(schedule_url).await?.text().await?;
    let json: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&resp)?;
    println!("Got json for sport {:?}", sport);

    let dates = get_array(&json, "dates")?;

    let mut out_games = Vec::new();
    if let Some(today) = dates.first() {

        let games = get_array_from_value(today, "games")?;

        for game in games {
            let status = get_object_from_value(game, "status")?;
            let detailed_state = status
                .get("detailedState")
                .ok_or("No detailed state present")?;
            if detailed_state == "Postponed" {
                continue;
            } else {
                let game_date = game.get("gameDate").ok_or("No game date present")?.as_str().ok_or("Date is not a string")?;
                println!("Got dame date {game_date}");
                let game_id = game.get("gamePk").ok_or("No game id present")?.as_u64().ok_or("Not an integer")?;

                let teams = get_object_from_value(game, "teams")?;

                let away_team_id = get_u64(get_object(get_object(teams, "away")?, "team")?, "id")?;
                let home_team_id= get_u64(get_object(get_object(teams, "home")?, "team")?, "id")?;

                let away_team = team_map.get(&away_team_id).ok_or(format!("Away team '{away_team_id}' not present"))?;
                let home_team= team_map.get(&home_team_id).ok_or(format!("Home team '{home_team_id}' not present"))?;

                let g = Game {
                        game_id,
                        sport_id: *sport,
                        home_team: Some(home_team.clone()),
                        away_team: Some(away_team.clone()),
                        home_score: 0,
                        away_score: 0,
                        status: Status::Active, // To be corrected later
                        ordinal: String::new(), 
                        start_time: DateTime::from_str(game_date)?,
                        extra: None
                    };
                out_games.push(g);
            }
        }
    }
    let results = join_all(out_games.into_iter().map(fetch_extra)).await;
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

async fn fetch_hockey(mut game: Game) -> Result<Game, Error> {
    println!("Fetching extra data for hockey game {:?}", game.game_id);
    let schedule_url = format!(
        "http://statsapi.web.nhl.com/api/v1/game/{}/linescore", game.game_id
    );

    let resp = reqwest::get(schedule_url).await?.text().await?;
    let json: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&resp)?;
    let teams = get_object(&json, "teams")?;
    let away = get_object(teams, "away")?;
    let home= get_object(teams, "home")?;

    game.away_score = get_u64(away, "goals").unwrap_or(0);
    game.home_score = get_u64(home, "goals").unwrap_or(0);

    let away_powerplay = get_bool(away, "powerPlay")?;
    let home_powerplay = get_bool(home, "powerPlay")?;
    let away_players = get_u64(away, "numSkaters")?;
    let home_players= get_u64(home, "numSkaters")?;
    let period = get_u64(&json, "currentPeriod")?;

    let period_time = get_str(&json, "currentPeriodTimeRemaining").unwrap_or("20:00");
    if period >= 1 {
        game.ordinal = get_str(&json, "currentPeriodOrdinal").unwrap_or("1st").to_string();
    }

    let mut status = Status::Invalid;
    if period_time == "Final" {
        status = Status::End;
    } else if period_time == "END" {
        if period >= 3 && game.away_score != game.home_score {
            status = Status::End;
        } else {
            status = Status::Intermission;
            game.ordinal += " INT";
        }
    } else if period_time == "20:00" && period > 1 {
        status = Status::Intermission;
        game.ordinal += " INT";
    } else if period_time == "20:00" && period >= 1 {
        status = Status::Active;
    } else {
        status = Status::Pregame;
    }

    game.status = status;
    game.extra = Some(ExtraGameData::HockeyData {
        away_powerplay,
        home_powerplay,
        away_players,
        home_players
    });

    println!("Got extra data for hockey game {:?}", game.game_id);
    Ok(game)
}