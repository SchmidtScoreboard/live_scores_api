use chrono::{DateTime, NaiveDateTime};
use itertools::Itertools;
use lazy_static::lazy_static;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use crate::common::data::{Error, ExtraGameData, Game, GolfPlayer, SportType, Status};

use crate::common::processors::{
    get_array_from_value, get_object, get_object_from_value, get_str, get_str_from_value, get_u64,
    get_u64_str_from_value,
};

pub fn process_golf(events: &Vec<Value>) -> Result<Vec<Game>, Error> {
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
            sport_id: SportType::Golf,
            home_team: None,
            away_team: None,
            home_score: 0,
            away_score: 0,
            status,
            period: 0,
            ordinal: ordinal.to_owned(),
            start_time: time,
            extra: Some(ExtraGameData::GolfData {
                players: top_5,
                name,
            }),
        })
    }
    Ok(out_games)
}
