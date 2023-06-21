use serde_json::Value;

use crate::common::data::{Error, ExtraGameData};

pub fn get_basketball_data(_competition: &Value) -> Result<ExtraGameData, Error> {
    Ok(ExtraGameData::BasketballData {})
}
