use std::str::FromStr;

use crate::common::data::Error;
use crate::common::types::game::Status;
use crate::common::types::sport::{Level, SportType};
use crate::common::types::Sport;

pub fn new_sport(sport_type: SportType, level: Level) -> Sport {
    let mut sport = Sport::default();
    sport.set_sport_type(sport_type);
    sport.set_level(level);
    sport
}

pub fn all_sports() -> Vec<Sport> {
    vec![
        new_sport(SportType::Hockey, Level::Professional),
        new_sport(SportType::Baseball, Level::Professional),
        new_sport(SportType::Golf, Level::Professional),
        new_sport(SportType::Basketball, Level::Professional),
        new_sport(SportType::Basketball, Level::Collegiate),
        new_sport(SportType::Football, Level::Professional),
        new_sport(SportType::Football, Level::Collegiate),
    ]
}

impl ToString for Sport {
    fn to_string(&self) -> String {
        match (self.sport_type(), self.level()) {
            (SportType::Hockey, Level::Professional) => "hockey".to_string(),
            (SportType::Baseball, Level::Professional) => "baseball".to_string(),
            (SportType::Golf, Level::Professional) => "golf".to_string(),
            (SportType::Basketball, Level::Professional) => "basketball".to_string(),
            (SportType::Basketball, Level::Collegiate) => "college-basketball".to_string(),
            (SportType::Football, Level::Professional) => "football".to_string(),
            (SportType::Football, Level::Collegiate) => "college-football".to_string(),
            _ => "".to_string(),
        }
    }
}

impl FromStr for Sport {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "golf" => Ok(new_sport(SportType::Golf, Level::Professional)),
            "baseball" => Ok(new_sport(SportType::Baseball, Level::Professional)),
            "hockey" => Ok(new_sport(SportType::Hockey, Level::Professional)),
            "football" => Ok(new_sport(SportType::Football, Level::Professional)),
            "college-football" => Ok(new_sport(SportType::Football, Level::Collegiate)),
            "basketball" => Ok(new_sport(SportType::Basketball, Level::Professional)),
            "college-basketball" => Ok(new_sport(SportType::Basketball, Level::Collegiate)),
            _ => Err(Error::InvalidSportType(s.to_string())),
        }
    }
}

pub fn from_espn(input: &str) -> Status {
    match input {
        "STATUS_IN_PROGRESS" => Status::Active,
        "STATUS_FINAL" | "STATUS_PLAY_COMPLETE" => Status::End,
        "STATUS_SCHEDULED" | "STATUS_RAIN_DELAY" => Status::Pregame,
        "STATUS_END_PERIOD" | "STATUS_HALFTIME" | "STATUS_DELAYED" => Status::Intermission,
        "STATUS_POSTPONED" | "STATUS_CANCELED" => Status::Invalid,
        _ => panic!("Unknown status {input}"),
    }
}
