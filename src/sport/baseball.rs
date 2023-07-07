use serde_json::Value;

use crate::common::data::Error;

use crate::common::processors::{
    get_bool, get_object, get_object_from_value, get_str, get_u64_str,
};
use crate::common::types::game::{BaseballData, SportData};

pub fn get_baseball_data(competition: &Value) -> Result<SportData, Error> {
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
    Ok(SportData::BaseballData(BaseballData {
        balls,
        outs,
        strikes,
        is_inning_top,
        on_first,
        on_second,
        on_third,
    }))
}
