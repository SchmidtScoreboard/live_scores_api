use live_sports::{fetch_all, SportType, Level, fetch_scores};
use std::collections::HashSet;
use std::env;

fn process_args() -> HashSet<SportType> {
    let args = env::args();
    let flags = args.skip(1);
    let mut set = HashSet::new();
    for flag in flags {
        let sport_type = match flag.as_str() {
            "golf" => SportType::Golf,
            "baseball" => SportType::Baseball,
            "hockey" => SportType::Hockey,
            "football" => SportType::Football(Level::Professional),
            "college-football" => SportType::Football(Level::College),
            "basketball" => SportType::Basketball(Level::Professional),
            "college-basketball" => SportType::Basketball(Level::College),
            "all" => return HashSet::new(),
            _ => panic!("Invalid flag: '{flag}'"),
        };
        set.insert(sport_type);
    }
    println!("Processed args, got {set:?}");
    set

}

#[tokio::main]
async fn main() -> Result<(), live_sports::Error> {
    let sports = process_args();
    let scores = match sports.len() {
        0 => fetch_all().await?,
        _ => fetch_scores(sports.clone()).await?
    };
    println!("Done fetching scores for {sports:?}\n{scores:?}");
    Ok(())
}
