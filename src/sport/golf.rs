use chrono::{DateTime, NaiveDateTime};
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use crate::common::data::Error;
use crate::common::types::game::golf_data::GolfPlayer;
use crate::common::types::game::{GolfData, SportData, Status};
use crate::common::types::Game;

use crate::common::processors::{
    get_array_from_value, get_object, get_object_from_value, get_str, get_str_from_value, get_u64,
    get_u64_str, get_u64_str_from_value,
};
use crate::{new_sport, Level, SportType};

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
        name: display_name.clone(),
        display_name,
        score,
        position,
    })
}
pub fn from_raw_data(line: &str, position: usize) -> Option<GolfPlayer> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#".*\s([a-zA-z ]+)/([a-zA-z ]+)\s*([^\s]+)+"#).unwrap();
    }
    if let Some(cap) = RE.captures_iter(line).next() {
        let player_a = &cap[0];
        let player_b = &cap[1];
        let score = cap[2].to_owned();
        let display_name = format!("{}/{}", &player_a[..5], &player_b[..5]);
        Some(GolfPlayer {
            name: display_name.clone(),
            display_name,
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
        .unwrap()
        .to_owned();
    let position = get_u64_str(
        get_object(get_object_from_value(competitor, "status")?, "position")?,
        "id",
    )?;
    Ok(GolfPlayer {
        name: full_name,
        display_name: last_name,
        position,
        score,
    })
}

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

pub fn process_golf(events: &Vec<Value>) -> Result<Vec<Game>, Error> {
    let mut out_games = Vec::new();

    for event in events {
        let competition = get_array_from_value(event, "competitions")?
            .first()
            .ok_or(format!("Missing competitions in {event}"))?;
        let competitors = get_array_from_value(competition, "competitors")?;
        let status_object = get_object_from_value(competition, "status")?;
        let espn_status = get_str(get_object(status_object, "type")?, "name")?;
        let mut status = from_espn(espn_status);
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
            tracing::info!(
                "Skipping event {} because it is {} hours old, status is {:?}",
                game_id,
                delta_hours,
                status
            );
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
                    .filter_map(|(position, line)| from_raw_data(line, position))
                    .take(5)
                    .collect();
            } else {
                // No raw data
                let competitors = get_array_from_value(competition, "competitors")?;
                let mut candidates = vec![];
                for competitor in competitors {
                    candidates.push(from_teamstroke(competitor)?)
                }
                candidates.sort_by(|a, b| a.position.cmp(&b.position));
                top_5 = candidates.into_iter().take(5).collect();
            }
        } else {
            let competitors = get_array_from_value(competition, "competitors")?;
            let mut candidates = vec![];
            for competitor in competitors {
                candidates.push(from_competitor(competitor)?)
            }
            candidates.sort_by(|a, b| a.position.cmp(&b.position));
            top_5 = candidates.into_iter().take(5).collect();
        }

        let mut name = get_str_from_value(event, "shortName")?.to_uppercase();
        tracing::info!("Raw name is {name}");
        lazy_static! {
            static ref NAME_MAP: HashMap<&'static str, &'static str> = {
                let mut m = HashMap::new();
                m.insert("SHRINERS CHILDREN'S OPEN", "SHRINERS OPEN");
                m.insert("BUTTERFIELD BERMUDA CHAMPIONSHIP", "BERMUDA CHAMP");
                m.insert(
                    "WORLD WIDE TECHNOLOGY CHAMPIONSHIP AT MAYAKOBA",
                    "WWT CHAMP",
                );
                m.insert("FARMERS INSURANCE OPEN", "FARMERS OPEN");
                m.insert("SONY OPEN IN HAWAII", "SONY OPEN");
                m.insert("AT&T PEBBLE BEACH PRO-AM", "PEBBLE BEACH");
                m.insert("WASTE MANAGEMENT PHOENIX OPEN", "WM PHOENIX");
                m.insert("CORALES PUNTACANA CHAMPIONSHIP", "PUTACANA CHAMP");
                m.insert("VALERO TEXAS OPEN", "VALERO OPEN");
                m.insert("RBC CANADIAN OPEN", "RBC CANADIAN");
                m.insert("GENESIS SCOTTISH OPEN", "SCOTTISH OPEN");
                m.insert("THE CJ CUP IN SOUTH CAROLINA", "CJ CUP");
                m.insert("CADENCE BANK HOUSTON OPEN", "HOUSTON OPEN");
                m
            };
        };
        if let Some(new_name) = NAME_MAP.get(&name as &str) {
            name = new_name.to_string()
        }

        lazy_static! {
            static ref DUMB_WORDS: HashSet<&'static str> = {
                let mut s = HashSet::new();
                s.insert("TOURNAMENT");
                s.insert("CHAMPIONSHIP");
                s.insert("CHALLENGE");
                s.insert("CLASSIC");
                s.insert("INVITATIONAL");
                s
            };
        };

        let name = name
            .split(' ')
            .filter(|word| {
                !DUMB_WORDS.contains(*word) // TODO remove numbers
            })
            .join(" ");

        out_games.push(Game {
            game_id,
            sport: Some(new_sport(SportType::Golf, Level::Professional)),
            home_team: None,
            away_team: None,
            home_team_score: 0,
            away_team_score: 0,
            status: status.into(),
            period: 0,
            ordinal: ordinal.to_owned(),
            start_time: time.timestamp_nanos(),
            sport_data: Some(SportData::GolfData(GolfData {
                event_name: name,
                players: top_5,
            })),
        })
    }
    Ok(out_games)
}
