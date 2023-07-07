extern crate prost_build;

fn main() {
    let mut config = prost_build::Config::new();
    config.type_attribute(
        "Sport",
        "#[derive(Eq, Hash, Copy, serde::Deserialize, serde::Serialize)]",
    );

    let output_attributes = "#[derive(serde::Serialize)]";
    config.type_attribute("Status", output_attributes);
    config.type_attribute(".common.types.game.SportData", output_attributes);
    config.type_attribute("BasketballData", output_attributes);
    config.type_attribute("BaseballData", output_attributes);
    config.type_attribute("FootballData", output_attributes);
    config.type_attribute("Possession", output_attributes);
    config.type_attribute("HockeyData", output_attributes);
    config.type_attribute("HockeyTeamData", output_attributes);
    config.type_attribute("GolfData", output_attributes);
    config.type_attribute("GolfPlayer", output_attributes);
    config.type_attribute("Game", output_attributes);
    config.type_attribute("Team", output_attributes);
    config.type_attribute("Color", output_attributes);
    config
        .compile_protos(&["src/common/types.proto"], &["src/"])
        .unwrap();
}
