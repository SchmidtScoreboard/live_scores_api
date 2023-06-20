use chrono::{serde::ts_seconds, ParseError};
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::str::FromStr;

use crate::common::team::Team;

use crate::common::processors::{
    get_array_from_value, get_object, get_object_from_value, get_str, get_str_from_value,
    get_u64_str,
};

#[derive(Clone, Debug, Eq, PartialEq, Hash, Copy, Serialize, Deserialize)]
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

impl Serialize for SportType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            SportType::Hockey => "hockey",
            SportType::Baseball => "baseball",
            SportType::Football(level) => match level {
                Level::Professional => "football",
                Level::College => "college-football",
            },
            SportType::Basketball(level) => match level {
                Level::Professional => "basketball",
                Level::College => "college-basketball",
            },
            SportType::Golf => "golf",
        })
    }
}

impl<'de> Deserialize<'de> for SportType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse()
            .map_err(|e| serde::de::Error::custom(format!("{e:?}")))
    }
}

impl FromStr for SportType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "golf" => Ok(SportType::Golf),
            "baseball" => Ok(SportType::Baseball),
            "hockey" => Ok(SportType::Hockey),
            "football" => Ok(SportType::Football(Level::Professional)),
            "college-football" => Ok(SportType::Football(Level::College)),
            "basketball" => Ok(SportType::Basketball(Level::Professional)),
            "college-basketball" => Ok(SportType::Basketball(Level::College)),
            _ => Err(Error::InvalidSportType(s.to_string())),
        }
    }
}

impl ToString for SportType {
    fn to_string(&self) -> String {
        match self {
            SportType::Golf => "golf".to_string(),
            SportType::Baseball => "baseball".to_string(),
            SportType::Hockey => "hockey".to_string(),
            SportType::Football(Level::Professional) => "football".to_string(),
            SportType::Football(Level::College) => "college-football".to_string(),
            SportType::Basketball(Level::Professional) => "basketball".to_string(),
            SportType::Basketball(Level::College) => "college-basketball".to_string(),
        }
    }
}

impl SportType {
    pub fn all() -> HashSet<SportType> {
        let mut set = HashSet::new();
        set.insert(SportType::Golf);
        set.insert(SportType::Baseball);
        set.insert(SportType::Hockey);
        set.insert(SportType::Football(Level::Professional));
        set.insert(SportType::Football(Level::College));
        set.insert(SportType::Basketball(Level::Professional));
        set.insert(SportType::Basketball(Level::College));
        set
    }

    pub fn all_vec() -> Vec<SportType> {
        SportType::all().into_iter().collect()
    }
}

#[derive(Debug)]
pub enum Error {
    FetchError(reqwest::Error),
    ParseError(String),
    InvalidSportType(String),
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
pub enum Status {
    Pregame,
    Active,
    Intermission,
    End,
    Invalid,
}

impl Status {
    pub fn from_espn(input: &str) -> Status {
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
    pub game_id: u64,
    pub sport_id: SportType,
    pub home_team: Option<Team>,
    pub away_team: Option<Team>,
    pub home_score: u64,
    pub away_score: u64,
    pub status: Status,
    pub period: u64,
    pub ordinal: String,
    #[serde(with = "ts_seconds")]
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub extra: Option<ExtraGameData>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Copy, Serialize, Deserialize)]
pub enum Possession {
    Home,
    Away,
    None,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct GolfPlayer {
    pub display_name: String,
    pub score: String,
    pub position: u64,
}

impl GolfPlayer {
    pub fn from_teamstroke(competitor: &Value) -> Result<GolfPlayer, Error> {
        let stats = get_array_from_value(competitor, "statistics")?;
        let score = if let Some(latest_stat) = stats.first() {
            get_str_from_value(latest_stat, "displayValue")?
        } else {
            "E"
        }
        .to_owned();
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
    pub fn from_raw_data(line: &str, position: usize) -> Option<GolfPlayer> {
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

    pub fn from_competitor(competitor: &Value) -> Result<GolfPlayer, Error> {
        let stats = get_array_from_value(competitor, "statistics")?;
        let score = if let Some(latest_stat) = stats.first() {
            get_str_from_value(latest_stat, "displayValue")?
        } else {
            "E"
        }
        .to_owned();

        lazy_static! {
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
        };

        let full_name =
            get_str(get_object_from_value(competitor, "athlete")?, "displayName")?.to_uppercase();
        let last_name = full_name
            .split(' ')
            .rev()
            .find(|s| !INVALID_NAMES.contains(s))
            .unwrap();
        let position = get_u64_str(
            get_object(get_object_from_value(competitor, "status")?, "position")?,
            "id",
        )?;
        Ok(GolfPlayer {
            display_name: last_name.to_owned(),
            position,
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
