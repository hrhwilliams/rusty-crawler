use std::collections::{HashMap, VecDeque};

use scraper::{Html, Selector};
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};
use url::Url;

const DB_URL: &str = "sqlite://crawler-graph.db";

struct Crawler {
    client: reqwest::Client,
    graph: HashMap::<String, Vec<String>>,
    queue: VecDeque<String>
}

type Result<T> = std::result::Result<T, CrawlerError>;

#[derive(Clone, Debug)]
enum CrawlerError {
    RequestError,
    EmptyQueue,
    GraphInsertError
}

impl Crawler {
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            graph: HashMap::<String, Vec<String>>::new(),
            queue: VecDeque::<String>::new()
        }
    }

    async fn explore_url(&mut self, url: String) -> Result<()> {
        if self.graph.contains_key(&url) {
            return Ok(());
        }

        let response = make_request(&self.client, &url).await
            .or(Err(CrawlerError::RequestError))?;

        let links = extract_urls(&url, &response);
        let mut links_deque = VecDeque::<String>::from(links.clone());
        self.queue.append(&mut links_deque);

        if self.graph.insert(url, links).is_none() {
            Ok(())
        } else {
            Err(CrawlerError::GraphInsertError)
        }
    }

    async fn explore_queue(&mut self) -> Result<()> {
        if let Some(url) = self.queue.pop_front() {
            self.explore_url(url).await?;
            Ok(())
        } else {
            Err(CrawlerError::EmptyQueue)
        }
    }
}

fn extract_urls(url: &str, body: &str) -> Vec<String> {
    let base_url = Url::parse(url).unwrap();
    let fragment = Html::parse_document(body);
    let a_selector = Selector::parse("a").unwrap();
    let mut links = Vec::<String>::new();

    for element in fragment.select(&a_selector) {
        if let Some(href) = element.value().attr("href") {
            if let Ok(href_url) = Url::parse(href) {
                links.push(href_url.to_string());
            } else if let Ok(href_url) = base_url.join(href) {
                links.push(href_url.to_string());
            }
        }
    }

    links
}

async fn make_request(client: &reqwest::Client, url: &str) -> Result<String>  {
    client.get(url)
        .send()
        .await.unwrap()
        .text()
        .await.or(Err(CrawlerError::RequestError))
}

async fn init_db() {
    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        Sqlite::create_database(DB_URL).await
            .expect("Failed to create database");
    } else {
        println!("Using database: {}", DB_URL);
    }

    let db = SqlitePool::connect(DB_URL).await.unwrap();
    let result = sqlx::query(
    "CREATE TABLE IF NOT EXISTS nodes (
         id INTEGER PRIMARY KEY AUTOINCREMENT,
         url TEXT NOT NULL);").execute(&db)
        .await
        .unwrap();
}

#[tokio::main]
async fn main() -> reqwest::Result<()>{
    let start = String::from("https://en.wikipedia.org/wiki/Main_Page");
    let mut crawler = Crawler::new();
    crawler.explore_url(start).await
        .expect("Failed to explore starting page");

    println!("{}" ,crawler.queue.len());

    for i in 0..10 {
        println!("{}", i);
        crawler.explore_queue().await
            .expect("Failed to explore from queue");
    }

    println!("Graph has {} explored nodes", crawler.graph.keys().len());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_urls() {
        // TODO
        // ignore mailto:, #, 
        let body = "
        <html><body>
          <h1>header</h1>
          <a href=\"https://www.example.com\">
          <a href=\"https://www.trailing-slash.com/\">
          <a href=\"https://www.example2.com\">
          <a href=\"/relative\">
          <a href=\"../other_rel\">
        </body></html>";

        let links = extract_urls("https://www.example.com/examples/", body);

        assert_eq!(links, [
            "https://www.example.com/",
            "https://www.trailing-slash.com/",
            "https://www.example2.com/",
            "https://www.example.com/relative", 
            "https://www.example.com/other_rel"
        ]);
    }

    #[test]
    fn test_extract_urls_ignores() {
        // TODO
        // ignore mailto:, #, 
        let body = "
        <html><body>
          <h1>header</h1>
          <a href=\"#item\">
          <a href=\"mailto:example@gmail.com\">
        </body></html>";

        let links = extract_urls("https://www.example.com/examples/", body);

        assert_eq!(links, Vec::<String>::new());
    }
}
