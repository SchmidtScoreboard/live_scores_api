extern crate phf;

mod color;
mod team;

use chrono::{DateTime, NaiveDateTime, ParseError, serde::ts_seconds};
use futures::future::join_all;
use itertools::Itertools;
use lazy_static::lazy_static;
use ordinal::Ordinal;
use regex::Regex;
use serde_json::{Map, Value};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use std::str::FromStr;
use team::{Team, BASEBALL_TEAMS, BASKETBALL_TEAMS, COLLEGE_TEAMS, FOOTBALL_TEAMS, HOCKEY_TEAMS};

#[derive(Clone, Debug, Eq, PartialEq, Hash, Copy, Serialize, Deserialize)]
pub enum Level {
    Professional,
    College,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Copy, Serialize, Deserialize)]
pub enum SportType {
    Hockey,
    Baseball,
    Football(Level),
    Basketball(Level),
    Golf,
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

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
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
    #[serde(with = "ts_seconds")]
    start_time: chrono::DateTime<chrono::Utc>,
    extra: Option<ExtraGameData>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Copy, Serialize, Deserialize)]
pub enum Possession {
    Home,
    Away,
    None,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct GolfPlayer {
    display_name: String,
    score: String,
    position: u64,
}

impl GolfPlayer {
    fn from_teamstroke(competitor: &Value) -> Result<GolfPlayer, Error> {
        let stats = get_array_from_value(competitor, "statistics")?;
        let score = if let Some(latest_stat) = stats.first() {
            get_str_from_value(latest_stat, "displayValue")?
        } else {
            "E"
        }.to_owned();
        let mut names = vec![];
        let roster = get_array_from_value(competitor, "roster")?;
        for player in roster {
            let last_name = &get_str(get_object_from_value(player, "athlete")?, "lastName")?[0..5];
            names.push(last_name);
        }
        let display_name = names.iter().join("/").to_uppercase();
        let position = get_u64_str(
            get_object(get_object_from_value(competitor, "status")?, "position")?,
            "id",
        )?;
        Ok(GolfPlayer {
            display_name,
            score,
            position,
        })
    }
    fn from_raw_data(line: &str, position: usize) -> Option<GolfPlayer> {
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r#".*\s([a-zA-z ]+)/([a-zA-z ]+)\s*([^\s]+)+"#).unwrap();
        }
        if let Some(cap) = RE.captures_iter(line).next() {
            let player_a = &cap[0];
            let player_b = &cap[1];
            let score = cap[2].to_owned();
            Some(GolfPlayer {
                display_name: format!("{}/{}", &player_a[..5], &player_b[..5]),
                score,
                position: position as u64,
            })
        } else {
            None
        }
    }

    fn from_competitor(competitor: &Value) -> Result<GolfPlayer, Error> {
        let stats = get_array_from_value(competitor, "statistics")?;
        let score = if let Some(latest_stat) = stats.first() {
            get_str_from_value(latest_stat, "displayValue")?
        } else {
            "E"
        }.to_owned();

        lazy_static!(
            static ref INVALID_NAMES: HashSet<&'static str> = {
                let mut s = HashSet::new();
                s.insert("JR.");
                s.insert("JR");
                s.insert("SR.");
                s.insert("SR");
                s.insert("II");
                s.insert("III");
                s.insert("IV");
                s.insert("V");
                s.insert("VI");
                s
            };
        );

        let full_name = get_str(get_object_from_value(competitor, "athlete")?, "displayName")?.to_uppercase();
        let last_name = full_name.split(' ').rev().find(|s| !INVALID_NAMES.contains(s)).unwrap();
        let position = get_u64_str(
            get_object(get_object_from_value(competitor, "status")?, "position")?,
            "id",
        )?;
        Ok(GolfPlayer {
            display_name: last_name.to_owned(),
            position: position as u64,
            score,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
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
    GolfData {
        players: Vec<GolfPlayer>,
        name: String,
    },
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
    let mut m = HashMap::new();
    for (sport, result) in results {
        let games = result?;
        m.insert(sport, games);
    }
    Ok(m)
}
pub async fn fetch_sport(sport: SportType) -> (SportType, Result<Vec<Game>, Error>) {
    (sport, match sport {
        SportType::Hockey => fetch_statsapi(&sport).await,
        _ => fetch_espn(&sport).await,
    })
}

async fn fetch_espn(sport: &SportType) -> Result<Vec<Game>, Error> {
    let url = get_espn_url(sport);
    let resp = reqwest::get(url).await?.text().await?;
    let json: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&resp)?;
    tracing::info!("Got json for sport {:?} at url {url}", sport);
    let events = get_array(&json, "events")?;

    if *sport == SportType::Golf {
        tracing::debug!("Doing golf stuff");
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
    let situation = get_object_from_value(competition, "situation");
    let status_object = get_object_from_value(competition, "status")?;

    let time_remaining = if game.status != Status::Active {
        ""
    } else {
        get_str(status_object, "displayClock").unwrap_or_default()
    }
    .to_owned();

    if let Ok(situation) = situation {

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
    } else {
        Ok(ExtraGameData::FootballData {
            time_remaining: "".to_owned(),
            ball_position: "".to_owned(),
            down_string: "".to_owned(),
            possession: Possession::None,
        })
    }
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
    let id = {
        let int_id = get_u64(team, "id");
        match int_id {
            Ok(id) => id,
            Err(_) => get_u64_str(team, "id")?
        }
    };
    let location = get_str(team, "location")?.to_owned();
    let name = get_str(team, "name")?.to_owned();
    let abbreviation = get_str(team, "abbreviation")?.to_owned();
    let display_name = get_display_name(&name);
    let primary_color = get_str(team, "color")?.to_owned();
    let secondary_color = get_str(team, "color").unwrap_or("000000");

    let secondary_color =
        color::get_secondary_for_primary(&primary_color, secondary_color)?.to_owned();
    let out = Team::new(
        id,
        location,
        name,
        display_name,
        abbreviation,
        primary_color,
        secondary_color,
    );

    tracing::info!("Creating unknown team: {:?}", out);
    Ok(out)
}

fn process_golf(events: &Vec<Value>) -> Result<Vec<Game>, Error> {
    let mut out_games = Vec::new();

    for event in events {
        let competition = get_array_from_value(event, "competitions")?
            .first()
            .ok_or(format!("Missing competitions in {event}"))?;
        let competitors = get_array_from_value(competition, "competitors")?;
        let status_object = get_object_from_value(competition, "status")?;
        let espn_status = get_str(get_object(status_object, "type")?, "name")?;
        let mut status = Status::from_espn(espn_status);
        if status == Status::Invalid {
            tracing::error!("Invalid status: {}", espn_status);
            continue;
        }

        let ordinal = format!("{}", get_u64(status_object, "period")?);
        let game_id = get_u64_str_from_value(competition, "id")?;

        let mut earliest_tee_time = None;
        for player in competitors {
            let player_status = get_object_from_value(player, "status")?;
            if let Ok(tee_time) = get_str(player_status, "teeTime") {
                let time = NaiveDateTime::parse_from_str(tee_time, "%Y-%m-%dT%H:%MZ")?;
                let time: DateTime<chrono::Utc> = DateTime::from_utc(time, chrono::Utc);
                if earliest_tee_time.map_or(true, |earliest| time < earliest) {
                    earliest_tee_time = Some(time);
                }
            }
        }
        let time = if let Some(e) = earliest_tee_time {
            e
        } else {
            let time_str = get_str_from_value(competition, "date")?;
            let time = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%dT%H:%MZ")?;
            DateTime::from_utc(time, chrono::Utc)
        };

        let now = chrono::offset::Utc::now();

        let delta_hours = now.signed_duration_since(time).num_hours().abs();
        tracing::info!("Now: {}, time: {}, delta_hours: {}", now, time, delta_hours);
        if delta_hours > 24 && !matches!(status, Status::Active | Status::End) {
            // skip events > 24 hours ago or in the future
            tracing::info!("Skipping event {} because it is {} hours old, status is {:?}", game_id, delta_hours, status);
            continue;
        }
        let scoring_system = get_object_from_value(competition, "scoringSystem")?;
        let scoring_system = get_str(scoring_system, "name")?;

        if status == Status::Active && time > now {
            // If tee time in the future, then this is after a day of play has ended
            status = Status::End;
        }

        let top_5: Vec<GolfPlayer>;
        if scoring_system == "Teamstroke" {
            if let Ok(raw_data) = get_str_from_value(competition, "rawData") {
                if status == Status::Active && raw_data.contains("COMPLETE") {
                    status = Status::End;
                }

                top_5 = raw_data
                    .split('\n')
                    .enumerate()
                    .filter_map(|(position, line)| GolfPlayer::from_raw_data(line, position))
                    .take(5)
                    .collect();
            } else {
                // No raw data
                let competitors = get_array_from_value(competition, "competitors")?;
                let mut candidates = vec![];
                for competitor in competitors {
                    candidates.push(GolfPlayer::from_teamstroke(competitor)?)
                }
                candidates.sort_by(|a, b| a.position.cmp(&b.position));
                top_5 = candidates.into_iter().take(5).collect();
            }
        } else {
            let competitors = get_array_from_value(competition, "competitors")?;
            let mut candidates = vec![];
            for competitor in competitors {
                candidates.push(GolfPlayer::from_competitor(competitor)?)
            }
            candidates.sort_by(|a, b| a.position.cmp(&b.position));
            top_5 = candidates.into_iter().take(5).collect();
        }

        let mut name = get_str_from_value(event, "shortName")?.to_uppercase();
        tracing::info!("Raw name is {name}");
        lazy_static!(
            static ref NAME_MAP: HashMap<&'static str, &'static str> = {
                let mut m = HashMap::new();
                m.insert("SHRINERS CHILDREN'S OPEN", "SHRINERS OPEN");
                m.insert("BUTTERFIELD BERMUDA CHAMPIONSHIP", "BERMUDA CHAMP");
                m.insert("WORLD WIDE TECHNOLOGY CHAMPIONSHIP AT MAYAKOBA", "WWT CHAMP");
                m.insert("FARMERS INSURANCE OPEN", "FARMERS OPEN");
                m.insert("SONY OPEN IN HAWAII", "SONY OPEN");
                m.insert("AT&T PEBBLE BEACH PRO-AM", "PEBBLE BEACH");
                m.insert("WASTE MANAGEMENT PHOENIX OPEN", "WM PHOENIX");
                m.insert("CORALES PUNTACANA CHAMPIONSHIP", "PUTACANA CHAMP");
                m.insert("VALERO TEXAS OPEN", "VALERO OPEN");
                m.insert("RBC CANADIAN OPEN", "RBC CANADIAN");
                m.insert("GENESIS SCOTTISH OPEN", "SCOTTISH OPEN");
                m
            };
        );
        if let Some(new_name) = NAME_MAP.get(&name as &str) {
            name = new_name.to_string()
        }

        lazy_static!(
            static ref DUMB_WORDS: HashSet<&'static str> = {
                let mut s = HashSet::new();
                s.insert("TOURNAMENT");
                s.insert("CHAMPIONSHIP");
                s.insert("CHALLENGE");
                s.insert("CLASSIC");
                s.insert("INVITATIONAL");
                s
            };
        );

        let name = name.split(' ').filter(|word| {
            !DUMB_WORDS.contains(*word) // TODO remove numbers
        }).join(" ");

        out_games.push(Game {
           game_id,
           sport_id: SportType::Golf,
           home_team: None,
           away_team: None,
           home_score: 0,
           away_score: 0,
           status,
           period: 0,
           ordinal: ordinal.to_owned(),
           start_time: time,
           extra: Some(ExtraGameData::GolfData { players: top_5, name})  
        })
    }
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
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object}"))?;
    let obj = value
        .as_object()
        .ok_or(format!("{name} is not an object {value}\nObject is {object}"))?;
    Ok(obj)
}
fn get_array_from_value<'a>(
    object: &'a Value,
    name: &'static str,
) -> Result<&'a Vec<Value>, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object}"))?;
    let arr = value.as_array().ok_or(format!("{name} is not an array {value}\nObject is {object}"))?;
    Ok(arr)
}
fn get_str_from_value<'a>(object: &'a Value, name: &'static str) -> Result<&'a str, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object}"))?;
    let str = value
        .as_str()
        .ok_or(format!("{name} is not a string {value:?}\nObject is {object}"))?;
    Ok(str)
}
fn get_u64_from_value(object: &Value, name: &'static str) -> Result<u64, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object}"))?;
    let num = value.as_u64().ok_or(format!("{name} is not an integer {value:?}\nObject is: {object}"))?;
    Ok(num)
}

fn get_u64_str_from_value(object: &Value, name: &'static str) -> Result<u64, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object}"))?;
    let str = value
        .as_str()
        .ok_or(format!("{name} is not a string {value:?}\nObject is: {object}"))?;
    let num = str
        .parse::<u64>()
        .map_err(|_| format!("{name} is not an integer from string {str}\nObject is: {object}"))?;
    Ok(num)
}

fn get_object<'a>(
    object: &'a Map<String, Value>,
    name: &'static str,
) -> Result<&'a Map<String, Value>, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object:?}"))?;
    let obj = value.as_object().ok_or(format!("{name} is not an object {value}\nObject is {object:?}"))?;
    Ok(obj)
}
fn get_array<'a>(
    object: &'a Map<String, Value>,
    name: &'static str,
) -> Result<&'a Vec<Value>, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object:?}"))?;
    let arr = value.as_array().ok_or(format!("{name} is not an array {value}\nObject is {object:?}"))?;
    Ok(arr)
}
fn get_str<'a>(object: &'a Map<String, Value>, name: &'static str) -> Result<&'a str, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object:?}"))?;
    let str = value.as_str().ok_or(format!("{name} is not a string {value}\nObject is {object:?}"))?; 
    Ok(str)
}
fn get_u64(object: &Map<String, Value>, name: &'static str) -> Result<u64, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object:?}"))?;
    let num = value.as_u64().ok_or(format!("{name} is not an integer {value:?}\nObject is {object:?}"))?;
    Ok(num)
}
fn get_u64_str(object: &Map<String, Value>, name: &'static str) -> Result<u64, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object:?}"))?;
    let str = value.as_str().ok_or(format!("{name} is not a string {value}\nObject is {object:?}"))?; 
    let num = str.parse::<u64>().map_err(|_| format!("{name} is not an integer from string {str}\nObject is: {object:?}"))?;
    Ok(num)
}
fn get_bool(object: &Map<String, Value>, name: &'static str) -> Result<bool, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object:?}"))?;
    let bool = value.as_bool().ok_or(format!("{name} is not a bool {value}\nObject is {object:?}"))?;
    Ok(bool)
}

async fn fetch_statsapi(sport: &SportType) -> Result<Vec<Game>, Error> {
    let team_map = &team::HOCKEY_TEAMS;
    let schedule_url = "http://statsapi.web.nhl.com/api/v1/schedule";

    let resp = reqwest::get(schedule_url).await?.text().await?;
    let json: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&resp)?;
    tracing::debug!("Got json for sport {:?}", sport);

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
                let game_date = get_str_from_value(game, "gameDate")?;
                println!("Got dame date {game_date}");
                let game_id = get_u64_from_value(game, "gamePk")?;

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
