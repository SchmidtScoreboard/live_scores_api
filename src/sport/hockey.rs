use crate::common::data::{Error, ExtraGameData, Game, Status};
use crate::common::processors::{get_bool, get_object, get_str, get_u64};

pub async fn fetch_hockey(mut game: Game) -> Result<Game, Error> {
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
    let away_players = get_u64(away, "numSkaters").unwrap_or(5);
    let home_players = get_u64(home, "numSkaters").unwrap_or(5);
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
