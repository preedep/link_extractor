use std::io::Error;

use clap::Parser;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::{debug, info};
use reqwest::Client;
use select::document::Document;
use select::predicate::Name;
/// Cli (Command Line Interface) for extracting link from yours web site.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    ///Your web site url
    #[arg(short, long)]
    pub url: String,
    ///Your proxy url
    #[arg(short, long)]
    pub proxy: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    let cli = Cli::parse();
    let url = cli.url;
    info!("open url: {}", &url);
    let client = Client::builder();
    if let Some(proxy) = cli.proxy.as_deref() {
        info!("with proxy: {}", proxy);
        let proxy = reqwest::Proxy::all(proxy).unwrap();
        let client = client.proxy(proxy).build().expect("TODO: panic message");
        extract_all_link(&url, &client).await;
    } else {
        let client = client.build().expect("TODO: panic message");
        extract_all_link(&url, &client).await;
    }
    Ok(())
}

async fn extract_all_link(url: &String, client: &Client) {
    debug!("extract_all_link: {}", url);

    let resp =
        client.get(url).header("User-Agent",
                               "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15").send().await;
    if let Ok(resp) = resp {
        let body = resp
            .text()
            .await
            .map_err(|e| Error::new(std::io::ErrorKind::Other, e));
        match body {
            Ok(text) => {
                let mut futures = FuturesUnordered::new();
                Document::from(text.as_str())
                    .find(Name("img"))
                    .filter_map(|n| n.attr("src"))
                    .for_each(|x| {
                        if !x.is_empty() {
                            let future = async move {
                                let full_url = format!("{}{}", url, x);
                                info!("Image: {}", &full_url);
                                let resp = client.get(&full_url).send().await;
                                if let Ok(resp) = resp {
                                    if resp.status().is_success() {
                                        info!("Success: {}", &full_url);
                                    } else {
                                        info!("Error: {}", &full_url);
                                    }
                                }
                            };
                            futures.push(future);
                        }
                    });
                while let Some(_) = futures.next().await {}
            }
            Err(e) => {
                panic!("Error: {}", e);
            }
        }
    }
}
