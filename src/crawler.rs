use std::collections::{HashMap, VecDeque};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use futures::future;
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct Crawler {
    #[serde(skip)]
    client: reqwest::Client,
    graph: HashMap::<String, Vec<String>>,
    queue: VecDeque<String>
}

pub type Result<T> = std::result::Result<T, CrawlerError>;

#[derive(Clone, Debug)]
pub enum CrawlerError {
    RequestError,
    EmptyQueue,
    UrlParseError,
}

impl Crawler {
    pub fn new(url: String) -> Self {
        let mut crawler = Crawler {
            client: reqwest::Client::new(),
            graph: HashMap::<String, Vec<String>>::new(),
            queue: VecDeque::<String>::new()
        };

        crawler.queue.push_front(url);
        crawler
    }

    pub async fn explore_url(&mut self, url: String) -> Result<()> {
        if let Ok(parsed_url) = Url::parse(&url) {
            let response = make_request(&self.client, parsed_url.as_str()).await
                .or(Err(CrawlerError::RequestError))?;

            let links = extract_hrefs_from(&parsed_url.to_string(), &response);
            let mut links_deque = VecDeque::<String>::from(links.clone());

            self.queue.append(&mut links_deque);
            self.graph.insert(url, links);
            Ok(())
        } else {
            Err(CrawlerError::UrlParseError)
        }
    }

    pub async fn explore_queue(&mut self, ignore_already_crawled: bool) -> Result<()> {
        if let Some(url) = self.queue.pop_front() {
            if !self.graph.contains_key(&url) || !ignore_already_crawled {
                self.explore_url(url).await?;
            }
            Ok(())
        } else {
            Err(CrawlerError::EmptyQueue)
        }
    }

    pub async fn explore_queue_multi(&mut self, n: usize) -> Result<()> {
        let urls = self.queue.drain(0..n);

        let responses = future::join_all(urls.map(|url| {
            let client = &self.client;
            async move {
                client.get(url).send().await
            }
        })).await;

        for response in responses {
            if let Ok(content) = response {
                let url = content.url().to_string();
                let body = content.text().await.unwrap();
                let links = extract_hrefs_from(&url, &body);
                let mut links_deque = VecDeque::<String>::from(links.clone());
                self.queue.append(&mut links_deque);
                self.graph.insert(url, links);
            }
        }

        Ok(())
    }

    pub fn add_to_queue(&mut self, url: String) {
        self.queue.push_back(url);
    }

    pub fn explored_nodes(&self) -> usize {
        self.graph.len()
    }
}

fn extract_hrefs_from(url: &str, body: &str) -> Vec<String> {
    let base_url = Url::parse(url).expect("Failed to parse URL");
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

        let links = extract_hrefs_from("https://www.example.com/examples/", body);

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

        let links = extract_hrefs_from("https://www.example.com/examples/", body);

        assert_eq!(links, ["https://www.example.com/examples/"]);
    }
}