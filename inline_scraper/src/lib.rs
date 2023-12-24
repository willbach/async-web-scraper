use std::collections::{HashMap, VecDeque};

use uqbar_process_lib::{send_response, await_message, Address, Message, get_payload, Payload, Response};

#[allow(dead_code)]
mod scraping_lib;
use scraping_lib::*;

wit_bindgen::generate!({
    path: "wit",
    world: "process",
    exports: {
        world: Component,
    },
});

// This process takes a URL and a depth, and scrapes all the pages reachable from that URL
struct Component;
impl Guest for Component {
    fn init(our: String) {
        let our = Address::from_str(&our).unwrap();
        println!("{:?}: start", our.process);

        let Ok(Message::Request { ipc, .. }) = await_message() else {
            panic!("ft_worker: got bad init message");
        };

        let params = serde_json::from_slice::<ScrapingParams>(&ipc)
            .expect("ft_worker: got unparseable init message");

        let mut scraped_pages: ScrapedPages = HashMap::new();
        let mut queue = VecDeque::new();
        let max_depth = params.depth;

        // Push the first URL onto the queue
        queue.push_back((params.url, 0));

        while let Some((url, depth)) = queue.pop_front() {
            // If we've already scraped this page, or we've reached the max depth, skip it
            if depth > max_depth || scraped_pages.contains_key(&url) {
                continue;
            }

            // Get the page, parse the links, push links into the queue, and store the page
            match scrape_link(&our.node, &url, depth).send_and_await_response(15) {
                Ok(message) => {
                    let Ok(Message::Response { .. }) = message else {
                        println!("ft_worker: got bad response");
                        continue;
                    };

                    let Some(payload) = get_payload() else {
                        println!("ft_worker: got bad response");
                        continue;
                    };

                    let Ok(page) = String::from_utf8(payload.bytes) else {
                        println!("ft_worker: error decoding scraped page");
                        continue;
                    };

                    let links = get_links(&page);
                    for link in links {
                        queue.push_back((link, depth + 1));
                    }

                    scraped_pages.insert(url, page);
                },
                Err(e) => {
                    println!("ft_worker: error scraping link: {}", e);
                },
            }
        }

        // Respond to the parent process with all the scraped pages
        match Response::new()
            .payload(Payload {
                mime: Some("application/json".to_string()),
                bytes: serde_json::to_vec(&scraped_pages).unwrap(),
            })
            .send() {
                Ok(_) => {},
                Err(e) => {
                    println!("ft_worker: error sending response: {}", e);
                },
            }

    }
}
