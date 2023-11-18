use std::collections::{HashMap, VecDeque};
use std::{fs, io};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
struct Crawler {
    #[serde(skip)]
    client: reqwest::Client,
    graph: HashMap::<String, Vec<String>>,
    queue: VecDeque<String>
}

type Result<T> = std::result::Result<T, CrawlerError>;

#[derive(Clone, Debug)]
enum CrawlerError {
    RequestError,
    EmptyQueue,
    UrlParseError,
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
        if let Ok(parsed_url) = Url::parse(&url) {
            let response = make_request(&self.client, parsed_url.as_str()).await
                .or(Err(CrawlerError::RequestError))?;

            let links = extract_hrefs_from(parsed_url, &response);
            let mut links_deque = VecDeque::<String>::from(links.clone());

            self.queue.append(&mut links_deque);
            self.graph.insert(url, links);
            Ok(())
        } else {
            Err(CrawlerError::UrlParseError)
        }
    }

    async fn explore_queue(&mut self, ignore_already_crawled: bool) -> Result<()> {
        if let Some(url) = self.queue.pop_front() {
            if !self.graph.contains_key(&url) || !ignore_already_crawled {
                self.explore_url(url).await?;
            }
            Ok(())
        } else {
            Err(CrawlerError::EmptyQueue)
        }
    }
}

fn extract_hrefs_from(base_url: Url, body: &str) -> Vec<String> {
    let fragment = Html::parse_document(body);
    let a_selector = Selector::parse("a").unwrap();

    fragment.select(&a_selector)
        .filter_map(|element| {
            element.value().attr("href").and_then(|href| {
                let mut href_url = if let Ok(url) = Url::parse(href) {
                    url
                } else {
                    base_url.join(href).ok()?
                };

                href_url.set_fragment(None);
                href_url.has_host().then(|| href_url.to_string())
            })
        })
        .collect()
}

async fn make_request(client: &reqwest::Client, url: &str) -> Result<String>  {
    client.get(url)
        .send()
        .await.unwrap()
        .text()
        .await.or(Err(CrawlerError::RequestError))
}

#[tokio::main]
async fn main() -> reqwest::Result<()>{
    let mut crawler: Crawler;

    if let Ok(crawler_json) = fs::File::open("crawler.json") {
        let reader = io::BufReader::new(crawler_json);
        crawler = serde_json::from_reader(reader)
            .expect("Error deserializing crawler from IO buffer");
    } else {
        let start = String::from("https://en.wikipedia.org/wiki/Main_Page");
        crawler = Crawler::new();
        crawler.explore_url(start).await
            .expect("Failed to explore starting page");
    }

    for i in 0..10 {
        println!("{}", i);
        crawler.explore_queue(true).await
            .expect("Failed to explore from queue");
    }

    let serialized = serde_json::to_string(&crawler).unwrap();
    std::fs::write("crawler.json", serialized)
        .expect("Failed to serialize crawler.");

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

        let links = extract_hrefs_from(Url::parse("https://www.example.com/examples/").unwrap(), body);

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
          <a href=\"#item1\">
          <a href=\"mailto:example@gmail.com\">
        </body></html>";

        let links = extract_hrefs_from(Url::parse("https://www.example.com/examples/").unwrap(), body);

        assert_eq!(links, ["https://www.example.com/examples/"]);
    }
}
