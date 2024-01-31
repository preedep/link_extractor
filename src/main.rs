use std::io::Error;

use clap::Parser;
use log::{debug, info};
use select::document::Document;
use select::predicate::Name;

/// Cli (Command Line Interface) for extracting link from yours web site.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    ///Your web site url
    #[arg(short, long)]
    pub url: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    let cli = Cli::parse();
    let url = cli.url;
    debug!("url: {}", &url);

    let client = reqwest::Client::new();
    let resp =
        client.get(&url).header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15").send().await;
    if let Ok(resp) = resp {
        let body = resp
            .text().await.map_err(|e| Error::new(std::io::ErrorKind::Other, e));
        match body {
            Ok(text) => {
                Document::from(text.as_str())
                    .find(Name("img"))
                    .filter_map(|n| n.attr("href"))
                    .for_each(|x|
                        if !x.is_empty() {
                            info!("Link: {}", x);
                        }
                    );
            }
            Err(e) => {
                panic!("Error: {}", e);
            }
        }
    }
    Ok(())
}
