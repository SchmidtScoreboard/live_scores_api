use chrono::{DateTime, NaiveDateTime};
use futures::future::join_all;
use itertools::Itertools;
use ordinal::Ordinal;
use serde_json::Value;
use std::str::FromStr;

use crate::common::team::{create_team, get_team_map};

use crate::common::data::Error;
use crate::common::types::game::SportData;
use crate::common::types::{game::Status, sport::Level, sport::SportType, Game, Sport};

use crate::common::processors::{
    get_array, get_array_from_value, get_object, get_object_from_value, get_str,
    get_str_from_value, get_u64, get_u64_from_value, get_u64_str, get_u64_str_from_value,
};

use crate::sport::baseball::get_baseball_data;
use crate::sport::basketball::get_basketball_data;
use crate::sport::football::get_football_data;
use crate::sport::golf::process_golf;
use crate::sport::hockey::fetch_hockey;

fn get_espn_url(sport: &Sport) -> &'static str {
    match (sport.sport_type(), sport.level()) {
        (SportType::Hockey, _) => panic!("Not allowed to use ESPN for hockey"),
        (SportType::Baseball, _) => "http://site.api.espn.com/apis/site/v2/sports/baseball/mlb/scoreboard",
        (SportType::Football, Level::Professional) => "http://site.api.espn.com/apis/site/v2/sports/football/nfl/scoreboard",
        (SportType::Football, Level::Collegiate) => "http://site.api.espn.com/apis/site/v2/sports/football/college-football/scoreboard?groups=80",
        (SportType::Basketball, Level::Professional) => "http://site.api.espn.com/apis/site/v2/sports/basketball/nba/scoreboard",
        (SportType::Basketball, Level::Collegiate)=> "http://site.api.espn.com/apis/site/v2/sports/basketball/mens-college-basketball/scoreboard?groups=50",
        (SportType::Golf, _)=> "http://site.api.espn.com/apis/site/v2/sports/golf/leaderboard?league=pga",
        (_, _) => panic!("Invalid state")
    }
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

pub async fn fetch_espn(sport: &Sport) -> Result<Vec<Game>, Error> {
    let url = get_espn_url(sport);
    let resp = reqwest::get(url).await?.text().await?;
    let json: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&resp)?;
    tracing::info!("Got json for sport {:?} at url {url}", sport);
    let events = get_array(&json, "events")?;

    if sport.sport_type() == SportType::Golf {
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
        let status = from_espn(espn_status);
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
            let g = Game {
                game_id,
                sport: Some(sport.clone()),
                home_team: None,
                away_team: None,
                home_team_score: 0,
                away_team_score: 0,
                period,
                status: status.into(),
                ordinal,
                start_time: time.timestamp_nanos(),
                sport_data: None,
            };
            // TODO
            // g.extra = Some(get_extra_data(competition, &g)?);
            g
        };
        out_games.push(out_game)
    }
    Ok(out_games)
}

pub async fn fetch_statsapi(sport: &Sport) -> Result<Vec<Game>, Error> {
    let team_map = get_team_map(sport);
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
                    sport: Some(sport.clone()),
                    home_team: None,
                    away_team: None,
                    home_team_score: 0,
                    away_team_score: 0,
                    period: 0,
                    status: Status::Active.into(), // Will be corrected later
                    ordinal: String::new(),
                    start_time: DateTime::<chrono::Utc>::from_str(game_date)?.timestamp_nanos(),
                    sport_data: None,
                };
                out_games.push(g);
            }
        }
    }
    let results = join_all(out_games.into_iter().map(fetch_hockey)).await;
    results.into_iter().collect()
}

fn get_extra_data(competition: &Value, game: &Game) -> Result<SportData, Error> {
    match game.sport.unwrap().sport_type() {
        SportType::Baseball => get_baseball_data(competition),
        SportType::Football => get_football_data(competition, game),
        SportType::Basketball => get_basketball_data(competition),
        SportType::Hockey | SportType::Golf => unreachable!(),
    }
}
