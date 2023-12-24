use std::collections::HashMap;

use serde_json::json;

use uqbar_process_lib::{
    await_message, get_payload, http, send_request, spawn, Address, Capabilities, Context, Message,
    OnExit, ProcessId, Request,
};

mod scraping_lib;
use scraping_lib::*;

wit_bindgen::generate!({
    path: "wit",
    world: "process",
    exports: {
        world: Component,
    },
});

struct Component;
impl Guest for Component {
    fn init(our: String) {
        println!("async_http_example: begin");
        let our = Address::from_str(&our).unwrap();

        // Store all of the pages that we scrape
        let mut scraped_pages: ScrapedPages = HashMap::new();

        // TODO: either scrape_inline() or scrape_link().send()

        loop {
            match await_message() {
                Ok(message) => match message {
                    Message::Response { context, .. } => {
                        handle_response(&our, &mut scraped_pages, context);
                    }
                    Message::Request { ipc, .. } => {
                        handle_request(&our, &mut scraped_pages, ipc);
                    }
                },
                Err(send_error) => {
                    println!("async_http_example: send error: {:?}", send_error);
                }
            }
        }
    }
}

// handle_response handles 2 types of responses:
// 1. A response from a sub-process that did scraping (in-line model)
// 2. A response from a scraping request sent directly to http_client (callback model)
fn handle_response(our: &Address, scraped_pages: &mut ScrapedPages, context: Option<Context>) {
    let Some(context) = context else {
        // Ignore responses with no context
        println!("async_http_example response: no context");
        return;
    };

    let Some(payload) = get_payload() else {
        // Ignore responses with no payload
        println!("async_http_example response: no payload");
        return;
    };

    let Ok(context) = serde_json::from_slice::<ScrapeContext>(&context) else {
        // Ignore responses with invalid context
        println!("async_http_example response: invalid context");
        return;
    };
    // Determine how to handle the response payload based on the context
    match context {
        ScrapeContext::InLine => {
            // This is a response from the scraper sub-process, the payload is a HashMap of (link, page)
            let Ok(payload) = serde_json::from_slice::<InLinePayload>(&payload.bytes) else {
                // Ignore responses with invalid payloads
                println!("async_http_example response: invalid scraper payload");
                return;
            };

            // Now we store all the scraped pages in our state
            for (url, page) in payload {
                scraped_pages.insert(url, page);
            }
        }
        ScrapeContext::Callback(ScrapingParams { url, depth }) => {
            // This is a response from the http_client, the payload is a web page
            let Ok(page) = String::from_utf8(payload.bytes) else {
                // Ignore responses with invalid payloads
                println!("async_http_example response: invalid page payload");
                return;
            };

            // If depth < 3, we get all of the links on the page and send a request for each one
            if depth < 3 {
                let links = get_links(&page);
                for link in links {
                    scrape_link(&our.node, &link, depth + 1).send();
                }
            }

            // Store this page
            scraped_pages.insert(url, page);
        }
    }
}

fn handle_request(our: &Address, scraped_pages: &mut ScrapedPages, ipc: Vec<u8>) {
    // Stubbed out in this example
    println!("async_http_example: request: {:?}", ipc);
}

fn scrape_inline(our: &Address, params: &ScrapingParams) {
    let Ok(worker_process_id) = spawn(
        None,
        "/inline_scraper.wasm".into(),
        OnExit::None, // can set message-on-panic here
        &Capabilities::All,
        false, // not public
    ) else {
        println!("scraper: failed to spawn inline scraper!");
        return;
    };

    Request::new()
        .target(Address {
            node: our.node.clone(),
            process: worker_process_id,
        })
        .ipc(json!(ScrapeContext::InLine).to_string().as_bytes().to_vec())
        .context(json!(ScrapeContext::InLine).to_string().as_bytes().to_vec()) // context is used for example
        .send();
}
