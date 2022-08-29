use live_sports::{fetch_all, SportType, fetch_scores};
use std::collections::HashSet;
use std::env;

fn process_args() -> HashSet<SportType> {
    let mut args = env::args();
    let arg0 = args.next().unwrap();
    let mut set = HashSet::new();
    for arg in args {
        if arg == "all" {
            return SportType::all();
        }
        match arg.parse::<SportType>() {
            Ok(sport) => { set.insert(sport); },
            Err(_) => {
                println!("Usage: {} [all] [sport]*", arg0);
                println!("  all: fetch all sports");
                println!("  sport: fetch only the specified sport(s)");
                std::process::exit(0);
            }
        }
    }
    tracing::info!("Processed args, got {set:?}");
    set

}

#[tokio::main]
async fn main() -> Result<(), live_sports::Error> {
    tracing_subscriber::fmt::init();
    let sports = process_args();
    let scores = match sports.len() {
        0 => fetch_all().await?,
        _ => fetch_scores(sports.clone()).await?
    };
    tracing::info!("Done fetching scores for {sports:?}\n{scores:?}");
    Ok(())
}
