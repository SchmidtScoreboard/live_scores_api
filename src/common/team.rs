use crate::common::data::Error;
use crate::common::processors::{get_object_from_value, get_str, get_u64, get_u64_str};
use crate::common::types::sport::{Level, SportType};
use crate::common::types::{Sport, Team};

use crate::common::color;
use itertools::Itertools;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;

pub fn get_team_map(sport: &Sport) -> &HashMap<u64, Team> {
    match (sport.sport_type(), sport.level()) {
        (SportType::Hockey, _) => &HOCKEY_TEAMS,
        (SportType::Baseball, _) => &BASEBALL_TEAMS,
        (_, Level::Collegiate) => &COLLEGE_TEAMS,
        (SportType::Football, Level::Professional) => &FOOTBALL_TEAMS,
        (SportType::Basketball, Level::Professional) => &BASKETBALL_TEAMS,
        (_, _) => panic!("unreachable"),
    }
}

pub fn create_team(competitor: &Value) -> Result<Team, Error> {
    let team = get_object_from_value(competitor, "team")?;
    let id = {
        let int_id = get_u64(team, "id");
        match int_id {
            Ok(id) => id,
            Err(_) => get_u64_str(team, "id")?,
        }
    };
    let location = get_str(team, "location")?.to_owned();
    let name = get_str(team, "name")?.to_owned();
    let abbreviation = get_str(team, "abbreviation")?.to_owned();
    let display_name = get_display_name(&name);
    let primary_color = get_str(team, "color")?.to_owned();
    let secondary_color = get_str(team, "color").unwrap_or("000000");

    let secondary_color = color::get_secondary_for_primary(&primary_color, secondary_color)?;

    let out = Team {
        id,
        location,
        name,
        display_name,
        abbreviation,
        primary_color: Some(color::get_rgb_from_hex(&primary_color)?),
        secondary_color: Some(secondary_color),
    };

    tracing::info!("Creating unknown team: {:?}", out);
    Ok(out)
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

fn get_teams(json: &str) -> HashMap<u64, Team> {
    let vec = serde_json::from_str::<Vec<Team>>(json).unwrap();
    vec.into_iter().map(|t| (t.id, t)).collect()
}

pub static BASEBALL_TEAMS: Lazy<HashMap<u64, Team>> =
    Lazy::new(|| get_teams(include_str!("teams/baseball.json")));

pub static HOCKEY_TEAMS: Lazy<HashMap<u64, Team>> =
    Lazy::new(|| get_teams(include_str!("teams/hockey.json")));

pub static FOOTBALL_TEAMS: Lazy<HashMap<u64, Team>> =
    Lazy::new(|| get_teams(include_str!("teams/football.json")));

pub static COLLEGE_TEAMS: Lazy<HashMap<u64, Team>> =
    Lazy::new(|| get_teams(include_str!("teams/collegiate.json")));

pub static BASKETBALL_TEAMS: Lazy<HashMap<u64, Team>> =
    Lazy::new(|| get_teams(include_str!("teams/basketball.json")));
