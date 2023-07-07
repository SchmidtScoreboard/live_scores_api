use std::io::Write;

use live_sports::common::proto_helpers::new_sport;
use live_sports::common::team;
use live_sports::common::types::sport::{Level, SportType};
use live_sports::common::types::Sport;

fn hex_to_rgb(hex: &str) -> Result<(u8, u8, u8), core::num::ParseIntError> {
    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;
    Ok((r, g, b))
}

fn color_message(color: (u8, u8, u8)) -> String {
    format!(
        "{{\n\t\tred: {}\n\t\tgreen: {}\n\t\tblue: {}\n\t}}",
        color.0, color.1, color.2
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sport_types = [
        new_sport(SportType::Hockey, Level::Professional),
        new_sport(SportType::Baseball, Level::Professional),
        new_sport(SportType::Basketball, Level::Professional),
        new_sport(SportType::Football, Level::Professional),
        new_sport(SportType::Football, Level::Collegiate),
    ];

    for sport in sport_types {
        let f = std::fs::File::create(format!("{sport:?}.textproto")).unwrap();
        let mut f = std::io::BufWriter::new(f);

        let teams = team::get_team_map(&sport);

        for team in teams.values() {
            f.write_all(format!("Team {{ \n\tid: {}\n\tlocation: \"{}\"\n\tname: \"{}\"\n\tdisplay_name: \"{}\"\n\tabbreviation: \"{}\"\n\tprimary_color {}\n\tsecondary_color: {}\n}}\n", 
            team.id, team.location, team.name, team.display_name, team.abbreviation, color_message(hex_to_rgb(&team.primary_color)?), color_message(hex_to_rgb(&team.secondary_color)?)).as_bytes())
                .unwrap();
        }
    }

    Ok(())
}
