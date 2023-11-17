use std::collections::{HashMap, VecDeque};

use scraper::{Html, Selector};
use url::Url;

struct Crawler {
    client: reqwest::Client,
    graph: HashMap::<String, Vec<String>>,
    queue: VecDeque<String>
}

impl Crawler {
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            graph: HashMap::<String, Vec<String>>::new(),
            queue: VecDeque::<String>::new()
        }
    }

    async fn explore_url(&mut self, url: &str) -> Result<bool, reqwest::Error> {
        let response = make_request(&self.client, url).await?;
        let base_url = Url::parse(url).unwrap();
        let links = extract_hrefs(&response);

        for link in &links {
            if let Ok(absolute_url) = Url::parse(link) {
                self.queue.push_back(absolute_url.to_string());
            } else {
                self.queue.push_back(base_url.join(link).unwrap().to_string());
            }
        }

        Ok(self.graph.insert(url.to_string(), links).is_some())
    }

    async fn explore_queue(&mut self) -> Result<bool, reqwest::Error> {
        let url = self.queue.pop_front();
        self.explore_url(&url.unwrap()).await
    }
}

fn extract_hrefs(body: &str) -> Vec<String> {
    let fragment = Html::parse_document(body);
    let a_selector = Selector::parse("a").unwrap();
    let mut links = Vec::<String>::new();

    for element in fragment.select(&a_selector) {
        if let Some(href) = element.value().attr("href") {
            links.push(href.into());
        }
    }

    links
}

async fn make_request(client: &reqwest::Client, url: &str) -> Result<String, reqwest::Error>  {
    client.get(url)
        .send()
        .await?
        .text()
        .await
}

#[tokio::main]
async fn main() -> reqwest::Result<()>{
    let start = "https://en.wikipedia.org/wiki/Main_Page";
    let mut crawler = Crawler::new();
    crawler.explore_url(&start).await
        .expect("Failed to explore starting page");

    for _ in 0..3 {
        crawler.explore_queue().await
            .expect("Failed to explore from queue");
    }

    println!("Graph has {} explored nodes", crawler.graph.keys().len());

    Ok(())
}

mod tests {
    use super::*;

}
