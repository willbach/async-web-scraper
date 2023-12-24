use std::collections::HashMap;
use serde_json::json;
use serde::{Serialize, Deserialize};

use uqbar_process_lib::{Request, Address, http, ProcessId};

pub type InLinePayload = HashMap<String, String>;

#[derive(Serialize, Deserialize)]
pub struct ScrapingParams {
    pub url: String,
    pub depth: u32,
}

#[derive(Serialize, Deserialize)]
pub enum ScrapeContext {
    InLine,
    Callback(ScrapingParams), // URL of the scraped page
}

pub type ScrapedPages = HashMap<String, String>;

pub fn scrape_link(our_node: &str, link: &str, depth: u32) -> Request {
    Request::new()
        .target(
            Address {
                node: our_node.to_string(),
                process: ProcessId::from_str("http_client:sys:uqbar").unwrap(),
            }
        )
        .ipc(json!(http::OutgoingHttpRequest {
            method: "GET".to_string(),
            url: link.to_string(),
            version: None,
            headers: HashMap::new(),
        }).to_string().as_bytes().to_vec())
        .context(json!(ScrapeContext::Callback(ScrapingParams {
            url: link.to_string(),
            depth,
        })).to_string().as_bytes().to_vec())
}

pub fn get_links(page: &str) -> Vec<String> {
    // Parse html page to get all href links
    page.lines()
        .filter_map(|line| {
            if let Some(link) = line.find("href=\"") {
                let link = &line[link + 6..];
                if let Some(end) = link.find("\"") {
                    let link = &link[..end];
                    if link.starts_with("http") {
                        return Some(link.to_string());
                    }
                }
            }
            None
        })
        .collect()
}
