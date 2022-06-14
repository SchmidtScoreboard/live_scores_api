extern crate phf;

mod team;
mod color;

use chrono::{DateTime, NaiveDateTime, ParseError};
use futures::future::join_all;
use itertools::Itertools;
use ordinal::Ordinal;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use team::{Team, BASEBALL_TEAMS, BASKETBALL_TEAMS, COLLEGE_TEAMS, FOOTBALL_TEAMS, HOCKEY_TEAMS};

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
    ChronoParseError(ParseError),
    ParseIntError(std::num::ParseIntError),
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
impl From<std::num::ParseIntError> for Error {
    fn from(pe: std::num::ParseIntError) -> Self {
        Self::ParseIntError(pe)
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

impl Status {
    fn from_espn(input: &str) -> Status {
        match input {
            "STATUS_IN_PROGRESS" => Status::Active,
            "STATUS_FINAL" | "STATUS_PLAY_COMPLETE" => Status::End,
            "STATUS_SCHEDULED" => Status::Pregame,
            "STATUS_END_PERIOD" | "STATUS_HALFTIME" | "STATUS_DELAYED" => Status::Intermission,
            "STATUS_POSTPONED" | "STATUS_CANCELED" => Status::Invalid,
            _ => panic!("Unknown status {input}"),
        }
    }
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
    period: u64,
    ordinal: String,
    start_time: chrono::DateTime<chrono::Utc>,
    extra: Option<ExtraGameData>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Copy)]
pub enum Possession {
    Home,
    Away,
    None,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExtraGameData {
    HockeyData {
        away_powerplay: bool,
        home_powerplay: bool,
        away_players: u64,
        home_players: u64,
    },
    BaseballData {
        balls: u64,
        outs: u64,
        strikes: u64,
        is_inning_top: bool,
        on_first: bool,
        on_second: bool,
        on_third: bool,
    },
    BasketballData {},
    FootballData {
        time_remaining: String,
        ball_position: String,
        down_string: String,
        possession: Possession,
    },
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
        SportType::Hockey => fetch_statsapi(&sport).await,
        _ => fetch_espn(&sport).await,
    }
    .map(|vec| (sport, vec))
}

async fn fetch_espn(sport: &SportType) -> Result<Vec<Game>, Error> {
    let url = get_espn_url(sport);
    let resp = reqwest::get(url).await?.text().await?;
    let json: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&resp)?;
    println!("Got json for sport {:?}", sport);
    let events = get_array(&json, "events")?;

    if *sport == SportType::Golf {
        println!("Doing golf stuff");
        return process_golf(events);
    }

    let mut out_games = Vec::new();

    for event in events {
        let competition = get_array_from_value(event, "competitions")?
            .first()
            .ok_or(format!("Missing competitions in {event}"))?;
        let competitors = get_array_from_value(competition, "competitors")?;
        let status_object = get_object_from_value(competition, "status")?;
        let (home_team, away_team) = competitors
            .iter()
            .collect_tuple()
            .ok_or("Failed to unwrap home team and away team")?;
        let espn_status = get_str(get_object(status_object, "type")?, "name")?;
        let status = Status::from_espn(espn_status);
        if status == Status::Invalid {
            continue;
        }

        let time_str = get_str_from_value(competition, "date")?;
        let time = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%dT%H:%MZ")?;
        let time: DateTime<chrono::Utc> = DateTime::from_utc(time, chrono::Utc);
        let now = chrono::offset::Utc::now();

        let delta_hours = now.signed_duration_since(time).num_hours().abs();
        if delta_hours > 12 {
            // skip games > 12 hours ago or in the future
            continue;
        }

        let period = get_u64(status_object, "period")?;
        let mut ordinal = Ordinal(period).to_string();
        if status == Status::Intermission {
            ordinal += " INT";
        }
        if espn_status == "STATUS_HALFTIME" {
            ordinal = "HALFTIME".to_owned();
        }

        let team_map = get_team_map(sport);
        let home_id = get_u64_str(get_object_from_value(home_team, "team")?, "id")?;
        let away_id = get_u64_str(get_object_from_value(away_team, "team")?, "id")?;
        let home = match team_map.get(&home_id) {
            Some(t) => t.clone(),
            None => create_team(home_team)?,
        };
        let away = match team_map.get(&away_id) {
            Some(t) => t.clone(),
            None => create_team(away_team)?,
        };

        let game_id = get_u64_str_from_value(competition, "id")?;
        let home_score = get_u64_str_from_value(home_team, "score")?;
        let away_score = get_u64_str_from_value(away_team, "score")?;

        let out_game = {
            let mut g = Game {
                game_id,
                sport_id: *sport,
                home_team: Some(home),
                away_team: Some(away),
                home_score,
                away_score,
                period,
                status,
                ordinal,
                start_time: time,
                extra: None,
            };
            g.extra = Some(get_extra_data(competition, &g)?);
            g
        };
        out_games.push(out_game)
    }
    Ok(out_games)
}

fn get_extra_data(competition: &Value, game: &Game) -> Result<ExtraGameData, Error> {
    match game.sport_id {
        SportType::Baseball => get_baseball_data(competition),
        SportType::Football(_) => get_football_data(competition, game),
        SportType::Basketball(_) => get_basketball_data(competition),
        SportType::Hockey | SportType::Golf => unreachable!(),
    }
}

fn get_baseball_data(competition: &Value) -> Result<ExtraGameData, Error> {
    let situation = get_object_from_value(competition, "situation");

    let (mut balls, mut strikes, mut outs) = (0, 0, 0);
    let (mut on_first, mut on_second, mut on_third) = (false, false, false);
    if let Ok(situation) = situation {
        balls = get_u64_str(situation, "balls").unwrap_or(0);
        strikes = get_u64_str(situation, "strikes").unwrap_or(0);
        outs = get_u64_str(situation, "outs").unwrap_or(0);

        on_first = get_bool(situation, "onFirst").unwrap_or(false);
        on_second = get_bool(situation, "onSecond").unwrap_or(false);
        on_third = get_bool(situation, "onThird").unwrap_or(false);
    }

    let status_object = get_object_from_value(competition, "status")?;
    let is_inning_top = get_str(get_object(status_object, "type")?, "shortDetail")?.contains("Top");
    Ok(ExtraGameData::BaseballData {
        balls,
        outs,
        strikes,
        is_inning_top,
        on_first,
        on_second,
        on_third,
    })
}
fn get_football_data(competition: &Value, game: &Game) -> Result<ExtraGameData, Error> {
    let situation = get_object_from_value(competition, "situation")?;
    let status_object = get_object_from_value(competition, "status")?;

    let time_remaining = if game.status != Status::Active {
        ""
    } else {
        get_str(status_object, "displayClock").unwrap_or_default()
    }
    .to_owned();

    let ball_position = get_str(situation, "possessionText")
        .unwrap_or_default()
        .to_owned();

    let down_string = {
        let s = get_str(situation, "shortDownDistanceText").unwrap_or_default();
        s.replace('&', "+")
    };

    let possession = if let Ok(possessing_team_id) = get_u64_str(situation, "possession") {
        if let (Some(home_team), Some(away_team)) = (&game.home_team, &game.away_team) {
            if home_team.id == possessing_team_id {
                Possession::Home
            } else if away_team.id == possessing_team_id {
                Possession::Away
            } else {
                Possession::None
            }
        } else {
            Possession::None
        }
    } else {
        Possession::None
    };

    Ok(ExtraGameData::FootballData {
        time_remaining,
        ball_position,
        down_string,
        possession,
    })
}
fn get_basketball_data(_competition: &Value) -> Result<ExtraGameData, Error> {
    Ok(ExtraGameData::BasketballData {})
}

fn get_display_name(raw: &str) -> String {
    if raw.len() > 11 {
        let mut words = raw.split(' ').collect_vec();
        if let Some(last) = words.last_mut() {
            if *last == "State" {
                *last = "St";
            }
        }
        if let Some(first) = words.first_mut() {
            *first = match *first {
                "North" => "N",
                "South" => "S",
                "West" => "W",
                "East" => "E",
                "Central" => "C",
                _ => *first,
            }
        }
        words.join(" ")
    } else {
        raw.to_owned()
    }
}

fn create_team(competitor: &Value) -> Result<Team, Error> {
    let team = get_object_from_value(competitor, "team")?;
    let id = get_u64(team, "id")?;
    let location = get_str(team, "location")?.to_owned();
    let name = get_str(team, "name")?.to_owned();
    let abbreviation = get_str(team, "abbreviation")?.to_owned();
    let display_name = get_display_name(&name);
    let primary_color = get_str(team, "color")?.to_owned();
    let secondary_color = get_str(team, "color").unwrap_or("000000");

    let secondary_color = color::get_secondary_for_primary(&primary_color, secondary_color)?.to_owned();
    let out = Team::new(
        id,
        location,
        name,
        display_name,
        abbreviation,
        primary_color,
        secondary_color,
    );

    println!("Creating unknown team: {:?}", out);
    Ok(out)
}

fn process_extra(event: &Value, game: Game) -> Result<Game, Error> {
    match game.sport_id {
        SportType::Baseball => todo!(),
        SportType::Football(_) => todo!(),
        SportType::Basketball(_) => todo!(),
        SportType::Hockey | SportType::Golf => unreachable!(),
    }
}

fn process_golf(events: &Vec<Value>) -> Result<Vec<Game>, Error> {
    let mut out_games = Vec::new();
    Ok(out_games)
}

fn get_team_map(sport: &SportType) -> &phf::Map<u64, Team> {
    match sport {
        SportType::Hockey => &HOCKEY_TEAMS,
        SportType::Baseball => &BASEBALL_TEAMS,
        SportType::Football(level) => {
            if *level == Level::College {
                &COLLEGE_TEAMS
            } else {
                &FOOTBALL_TEAMS
            }
        }
        SportType::Basketball(level) => {
            if *level == Level::College {
                &COLLEGE_TEAMS
            } else {
                &BASKETBALL_TEAMS
            }
        }
        SportType::Golf => unreachable!(),
    }
}

fn get_espn_url(sport: &SportType) -> &'static str {
    match sport {
        SportType::Hockey => panic!("Not allowed to use ESPN for hockey"),
        SportType::Baseball => "http://site.api.espn.com/apis/site/v2/sports/baseball/mlb/scoreboard",
        SportType::Football(level) => match level {
            Level::Professional => "http://site.api.espn.com/apis/site/v2/sports/football/nfl/scoreboard",
            Level::College => "http://site.api.espn.com/apis/site/v2/sports/football/college-football/scoreboard?groups=80",
        }
        SportType::Basketball(level) => match level {
            Level::Professional => "http://site.api.espn.com/apis/site/v2/sports/basketball/nba/scoreboard",
            Level::College => "http://site.api.espn.com/apis/site/v2/sports/basketball/mens-college-basketball/scoreboard?groups=50",
        }
        SportType::Golf=> "http://site.api.espn.com/apis/site/v2/sports/golf/leaderboard?league=pga",
    }
}
fn get_object_from_value<'a>(
    object: &'a Value,
    name: &'static str,
) -> Result<&'a Map<String, Value>, Error> {
    Ok(object
        .get(name)
        .ok_or(format!("{name} not present {object}"))?
        .as_object()
        .ok_or(format!("{name} is not an object"))?)
}
fn get_array_from_value<'a>(
    object: &'a Value,
    name: &'static str,
) -> Result<&'a Vec<Value>, Error> {
    Ok(object
        .get(name)
        .ok_or(format!("{name} not present {object}"))?
        .as_array()
        .ok_or(format!("{name} is not an array"))?)
}
fn get_str_from_value<'a>(object: &'a Value, name: &'static str) -> Result<&'a str, Error> {
    Ok(object
        .get(name)
        .ok_or(format!("{name} not present {object:?}"))?
        .as_str()
        .ok_or(format!("{name} is not a string"))?)
}
fn get_u64_from_value(object: &Value, name: &'static str) -> Result<u64, Error> {
    Ok(object
        .get(name)
        .ok_or(format!("{name} not present {object:?}"))?
        .as_u64()
        .ok_or(format!("{name} is not an integer"))?)
}
fn get_u64_str_from_value(object: &Value, name: &'static str) -> Result<u64, Error> {
    Ok(object
        .get(name)
        .ok_or(format!("{name} not present {object:?}"))?
        .as_str()
        .ok_or(format!("{name} is not a string"))?
        .parse()
        .map_err(|e| format!("Failed to parse {name}: '{e:?}'"))?)
}

fn get_object<'a>(
    object: &'a Map<String, Value>,
    name: &'static str,
) -> Result<&'a Map<String, Value>, Error> {
    Ok(object
        .get(name)
        .ok_or(format!("{name} not present {object:?}"))?
        .as_object()
        .ok_or(format!("{name} is not an object"))?)
}
fn get_array<'a>(
    object: &'a Map<String, Value>,
    name: &'static str,
) -> Result<&'a Vec<Value>, Error> {
    Ok(object
        .get(name)
        .ok_or(format!("{name} not present {object:?}"))?
        .as_array()
        .ok_or(format!("{name} is not an array"))?)
}
fn get_str<'a>(object: &'a Map<String, Value>, name: &'static str) -> Result<&'a str, Error> {
    Ok(object
        .get(name)
        .ok_or(format!("{name} not present {object:?}"))?
        .as_str()
        .ok_or(format!("{name} is not a string"))?)
}
fn get_u64(object: &Map<String, Value>, name: &'static str) -> Result<u64, Error> {
    Ok(object
        .get(name)
        .ok_or(format!("{name} not present {object:?}"))?
        .as_u64()
        .ok_or(format!("{name} is not an integer"))?)
}
fn get_u64_str(object: &Map<String, Value>, name: &'static str) -> Result<u64, Error> {
    Ok(object
        .get(name)
        .ok_or(format!("{name} not present {object:?}"))?
        .as_str()
        .ok_or(format!("{name} is not a string"))?
        .parse()
        .map_err(|e| format!("Failed to parse {name}: '{e:?}'"))?)
}
fn get_bool(object: &Map<String, Value>, name: &'static str) -> Result<bool, Error> {
    Ok(object
        .get(name)
        .ok_or(format!("{name} not present {object:?}"))?
        .as_bool()
        .ok_or(format!("{name} is not a boolean"))?)
}

async fn fetch_statsapi(sport: &SportType) -> Result<Vec<Game>, Error> {
    let team_map = &team::HOCKEY_TEAMS;
    let schedule_url = "http://statsapi.web.nhl.com/api/v1/schedule";

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
                let game_date = game
                    .get("gameDate")
                    .ok_or("No game date present")?
                    .as_str()
                    .ok_or("Date is not a string")?;
                println!("Got dame date {game_date}");
                let game_id = game
                    .get("gamePk")
                    .ok_or("No game id present")?
                    .as_u64()
                    .ok_or("Not an integer")?;

                let teams = get_object_from_value(game, "teams")?;

                let away_team_id = get_u64(get_object(get_object(teams, "away")?, "team")?, "id")?;
                let home_team_id = get_u64(get_object(get_object(teams, "home")?, "team")?, "id")?;

                let away_team = team_map
                    .get(&away_team_id)
                    .ok_or(format!("Away team '{away_team_id}' not present"))?;
                let home_team = team_map
                    .get(&home_team_id)
                    .ok_or(format!("Home team '{home_team_id}' not present"))?;

                let g = Game {
                    game_id,
                    sport_id: *sport,
                    home_team: Some(home_team.clone()),
                    away_team: Some(away_team.clone()),
                    home_score: 0,
                    away_score: 0,
                    status: Status::Active, // To be corrected later
                    period: 0,
                    ordinal: String::new(),
                    start_time: DateTime::from_str(game_date)?,
                    extra: None,
                };
                out_games.push(g);
            }
        }
    }
    let results = join_all(out_games.into_iter().map(fetch_hockey)).await;
    results.into_iter().collect()
}

// async fn process_baseball(mut game: Game) -> Result<Game, Error> {

//     let resp = reqwest::get(schedule_url).await?.text().await?;
//     let json: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&resp)?;
//     let linescore = get_object(get_object(&json, "liveData")?, "linescore")?;
//     let teams = get_object(linescore, "teams")?;
//     let away = get_object(teams, "away")?;
//     let home= get_object(teams, "home")?;

//     game.away_score = get_u64(away, "goals").unwrap_or(0);
//     game.home_score = get_u64(home, "goals").unwrap_or(0);

//     let inning = get_u64(linescore, "currentInning").unwrap_or(0);
//     let is_inning_top = get_bool(linescore, "isInningTop").unwrap_or(false);

//     let state = get_str(get_object(get_object(&json, "gameData")?, "status")?, "abstractGameState")?;
//     if state == "Final" {
//         game.ordinal = get_str(linescore, "currentInningOrdinal").unwrap_or("FINAL").to_owned();
//         game.status = Status::End;
//     } else if state == "Live" {
//         game.ordinal = get_str(linescore, "currentInningOrdinal").unwrap_or("").to_owned();
//         game.status = Status::Active;
//     } else if state == "Preview" {
//         game.ordinal = String::new();
//         game.status = Status::Pregame;
//     } else {
//         game.status = Status::Invalid;
//     }

//     let mut balls = 0;
//     let mut strikes = 0;
//     let mut outs = 0;
//     if game.status == Status::Active {
//         balls = get_u64(linescore, "balls")?;
//         outs = get_u64(linescore, "outs")?;
//         strikes= get_u64(linescore, "strikes")?;
//         if outs == 3 {
//             if inning >= 9 && ((is_inning_top && game.home_score > game.away_score) || (!is_inning_top && game.home_score != game.away_score)) {
//                 game.ordinal = get_str(linescore, "currentInningOrdinal").unwrap_or("FINAL").to_owned();
//                 game.status = Status::End;
//             } else {
//                 game.ordinal = format!("Middle {}", game.ordinal);
//                 game.status = Status::Intermission;
//             }
//         }
//     }
//     game.extra = Some(ExtraGameData::BaseballData {
//         balls,
//         outs,
//         strikes,
//         inning,
//         is_inning_top
//      });

//     Ok(game)
// }

async fn fetch_hockey(mut game: Game) -> Result<Game, Error> {
    println!("Fetching extra data for hockey game {:?}", game.game_id);
    let schedule_url = format!(
        "http://statsapi.web.nhl.com/api/v1/game/{}/linescore",
        game.game_id
    );

    let resp = reqwest::get(schedule_url).await?.text().await?;
    let json: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&resp)?;
    let teams = get_object(&json, "teams")?;
    let away = get_object(teams, "away")?;
    let home = get_object(teams, "home")?;

    game.away_score = get_u64(away, "goals").unwrap_or(0);
    game.home_score = get_u64(home, "goals").unwrap_or(0);

    let away_powerplay = get_bool(away, "powerPlay")?;
    let home_powerplay = get_bool(home, "powerPlay")?;
    let away_players = get_u64(away, "numSkaters")?;
    let home_players = get_u64(home, "numSkaters")?;
    let period = get_u64(&json, "currentPeriod")?;
    game.period = period;

    let period_time = get_str(&json, "currentPeriodTimeRemaining").unwrap_or("20:00");
    if period >= 1 {
        game.ordinal = get_str(&json, "currentPeriodOrdinal")
            .unwrap_or("1st")
            .to_string();
    }

    let status = if period_time == "Final" {
        Status::End
    } else if period_time == "END" {
        if period >= 3 && game.away_score != game.home_score {
            Status::End
        } else {
            game.ordinal += " INT";
            Status::Intermission
        }
    } else if period_time == "20:00" && period > 1 {
        game.ordinal += " INT";
        Status::Intermission
    } else if period_time == "20:00" && period >= 1 {
        Status::Active
    } else {
        Status::Pregame
    };

    game.status = status;
    game.extra = Some(ExtraGameData::HockeyData {
        away_powerplay,
        home_powerplay,
        away_players,
        home_players,
    });

    println!("Got extra data for hockey game {:?}", game.game_id);
    Ok(game)
}
