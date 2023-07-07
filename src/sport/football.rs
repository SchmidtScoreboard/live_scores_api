use serde_json::Value;

use crate::common::data::Error;

use crate::common::processors::{get_object_from_value, get_str, get_u64_str};
use crate::common::types::game::football_data::Possession;
use crate::common::types::game::{FootballData, SportData, Status};
use crate::common::types::Game;

pub fn get_football_data(competition: &Value, game: &Game) -> Result<SportData, Error> {
    let situation = get_object_from_value(competition, "situation");
    let status_object = get_object_from_value(competition, "status")?;

    let time_remaining = if game.status() != Status::Active {
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

        Ok(SportData::FootballData(FootballData {
            time_remaining,
            ball_position,
            down_string,
            possession: possession.into(),
        }))
    } else {
        Ok(SportData::FootballData(FootballData {
            time_remaining: "".to_owned(),
            ball_position: "".to_owned(),
            down_string: "".to_owned(),
            possession: Possession::None.into(),
        }))
    }
}
