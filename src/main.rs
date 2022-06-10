use live_sports::fetch_all;


#[tokio::main]
async fn main() -> Result<(), live_sports::Error>{
    println!("Fetching all scores!");
    let scores = fetch_all().await?;
    println!("Done fetching all scores {:?}", scores);
    Ok(())
}
