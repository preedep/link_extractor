use std::io::Error;
use std::time::Instant;

use clap::Parser;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::{debug, error, info};
use reqwest::Client;
use select::document::Document;
use select::predicate::Name;
use url::Url;

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
    ///Html Tag , default is img
    #[arg(short, long, default_value = "img")]
    pub tag: Option<String>,
    ///Attribute name of Tag , default is src
    #[arg(short, long, default_value = "src")]
    pub attr: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    /*
    let logger = pretty_env_logger::formatted_builder().build();
    let multi = MultiProgress::new();

    LogWrapper::new(multi.clone(), logger)
        .try_init()
        .unwrap();
    */
    pretty_env_logger::init();

    let cli = Cli::parse();
    let url = cli.url;
    let tag = cli.tag.unwrap_or("img".to_string());
    let attr = cli.attr.unwrap_or("src".to_string());

    info!("open url: {}", &url);
    let client = Client::builder();
    if let Some(proxy) = cli.proxy.as_deref() {
        info!("with proxy: {}", proxy);
        let proxy = reqwest::Proxy::all(proxy).unwrap();
        let client = client.proxy(proxy).build().expect("TODO: panic message");
        extract_all_link(&url, &tag, &attr, &client).await;
    } else {
        let client = client.build().expect("TODO: panic message");
        extract_all_link(&url, &tag, &attr, &client).await;
    }
    Ok(())
}

async fn extract_all_link(url: &String, tag: &String, attr: &String, client: &Client) {
    //debug!("extract_all_link: {}", url);

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
                let doc = Document::from(text.as_str());
                print_links(url, tag, attr, client, &doc).await;
            }
            Err(e) => {
                panic!("Error: {}", e);
            }
        }
    }
}

async fn print_links(url: &String, tag: &String, attr: &String, client: &Client, doc: &Document) {
    let mut futures = FuturesUnordered::new();
    doc.find(Name(tag.as_str()))
        .filter_map(|n| n.attr(attr.as_str()))
        .for_each(|x| {
            if !x.is_empty() {
                let future = async move {

                    let full_url = format!("{}", url);
                    let url_info = Url::parse(&full_url).unwrap();
                    let resource_url = format!("{}://{}{}",
                                               url_info.scheme(),
                                               url_info.host_str().unwrap(),
                                               x);
                    let start = Instant::now();
                    let resp = client.get(&resource_url).send().await;
                    if let Ok(resp) = resp {
                        if resp.status().is_success() {
                            info!("Success: {}", &resource_url);

                            let cache_info = resp.headers().get("x-iinfo");
                            if let Some(cache_info) = cache_info {
                                info!("x-iinfo: {}", cache_info.to_str().unwrap());
                            }
                        } else {
                            error!("Error: {}", &full_url);
                        }
                        let elapsed = start.elapsed();
                        let message = format!("Download use Elapsed time: {:.2?}", elapsed);
                        info!("{}", message);
                    }

                };
                futures.push(future);
            }
        });

    while let Some(_) = futures.next().await {}
}
