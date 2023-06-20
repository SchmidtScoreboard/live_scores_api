use serde_json::{Map, Value};

use crate::common::data::Error;

pub fn get_object_from_value<'a>(
    object: &'a Value,
    name: &'static str,
) -> Result<&'a Map<String, Value>, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object}"))?;
    let obj = value.as_object().ok_or(format!(
        "{name} is not an object {value}\nObject is {object}"
    ))?;
    Ok(obj)
}
pub fn get_array_from_value<'a>(
    object: &'a Value,
    name: &'static str,
) -> Result<&'a Vec<Value>, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object}"))?;
    let arr = value.as_array().ok_or(format!(
        "{name} is not an array {value}\nObject is {object}"
    ))?;
    Ok(arr)
}
pub fn get_str_from_value<'a>(object: &'a Value, name: &'static str) -> Result<&'a str, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object}"))?;
    let str = value.as_str().ok_or(format!(
        "{name} is not a string {value:?}\nObject is {object}"
    ))?;
    Ok(str)
}
pub fn get_u64_from_value(object: &Value, name: &'static str) -> Result<u64, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object}"))?;
    let num = value.as_u64().ok_or(format!(
        "{name} is not an integer {value:?}\nObject is: {object}"
    ))?;
    Ok(num)
}

pub fn get_u64_str_from_value(object: &Value, name: &'static str) -> Result<u64, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object}"))?;
    let str = value.as_str().ok_or(format!(
        "{name} is not a string {value:?}\nObject is: {object}"
    ))?;
    let num = str
        .parse::<u64>()
        .map_err(|_| format!("{name} is not an integer from string {str}\nObject is: {object}"))?;
    Ok(num)
}

pub fn get_object<'a>(
    object: &'a Map<String, Value>,
    name: &'static str,
) -> Result<&'a Map<String, Value>, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object:?}"))?;
    let obj = value.as_object().ok_or(format!(
        "{name} is not an object {value}\nObject is {object:?}"
    ))?;
    Ok(obj)
}
pub fn get_array<'a>(
    object: &'a Map<String, Value>,
    name: &'static str,
) -> Result<&'a Vec<Value>, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object:?}"))?;
    let arr = value.as_array().ok_or(format!(
        "{name} is not an array {value}\nObject is {object:?}"
    ))?;
    Ok(arr)
}
pub fn get_str<'a>(object: &'a Map<String, Value>, name: &'static str) -> Result<&'a str, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object:?}"))?;
    let str = value.as_str().ok_or(format!(
        "{name} is not a string {value}\nObject is {object:?}"
    ))?;
    Ok(str)
}
pub fn get_u64(object: &Map<String, Value>, name: &'static str) -> Result<u64, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object:?}"))?;
    let num = value.as_u64().ok_or(format!(
        "{name} is not an integer {value:?}\nObject is {object:?}"
    ))?;
    Ok(num)
}
pub fn get_u64_str(object: &Map<String, Value>, name: &'static str) -> Result<u64, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object:?}"))?;
    let str = value.as_str().ok_or(format!(
        "{name} is not a string {value}\nObject is {object:?}"
    ))?;
    let num = str.parse::<u64>().map_err(|_| {
        format!("{name} is not an integer from string {str}\nObject is: {object:?}")
    })?;
    Ok(num)
}
pub fn get_bool(object: &Map<String, Value>, name: &'static str) -> Result<bool, Error> {
    let value = object
        .get(name)
        .ok_or(format!("{name} not present in {object:?}"))?;
    let bool = value.as_bool().ok_or(format!(
        "{name} is not a bool {value}\nObject is {object:?}"
    ))?;
    Ok(bool)
}
