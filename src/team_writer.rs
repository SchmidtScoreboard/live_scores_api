use itertools::Itertools;
use live_sports::common::proto_helpers::new_sport;
use live_sports::common::team;
use live_sports::common::types::sport::{Level, SportType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sport_types = [
        new_sport(SportType::Hockey, Level::Professional),
        new_sport(SportType::Baseball, Level::Professional),
        new_sport(SportType::Basketball, Level::Professional),
        new_sport(SportType::Football, Level::Professional),
        new_sport(SportType::Football, Level::Collegiate),
    ];

    for sport in sport_types {
        let f = std::fs::File::create(format!("{sport:?}.json")).unwrap();
        let f = std::io::BufWriter::new(f);

        let teams = team::get_team_map(&sport);
        serde_json::to_writer_pretty(f, &teams.values().collect_vec())?;
    }

    Ok(())
}
