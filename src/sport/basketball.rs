use serde_json::Value;

use crate::common::data::Error;
use crate::common::types::game::{BasketballData, SportData};

pub fn get_basketball_data(_competition: &Value) -> Result<SportData, Error> {
    Ok(SportData::BasketballData(BasketballData {}))
}
