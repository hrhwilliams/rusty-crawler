use std::{fs, io};

pub mod crawler;
use crawler::Crawler;

#[tokio::main]
async fn main() -> reqwest::Result<()>{
    let mut crawler: Crawler = if let Ok(crawler_json) = fs::File::open("crawler.json") {
        let reader = io::BufReader::new(crawler_json);
        serde_json::from_reader(reader)
            .expect("Error deserializing crawler from IO buffer")
    } else {
        Crawler::new("https://en.wikipedia.org/wiki/Main_Page".to_string())
    };

    for i in 0..1 {
        println!("{}", i);
        crawler.explore_queue(true).await
            .expect("Failed to explore from queue");
    }

    crawler.explore_queue_multi(100).await
        .expect("Failed to make async requests");

    println!("crawler has {} nodes", crawler.explored_nodes());

    let serialized = serde_json::to_string(&crawler).unwrap();
    std::fs::write("crawler.json", serialized)
        .expect("Failed to serialize crawler.");

    Ok(())
}
