use std::io::Error;
use std::time::Instant;

use clap::Parser;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::{error, info};
use regex::Regex;
use reqwest::Client;
use select::document::Document;
use select::predicate::Name;
use url::Url;

type ReqAndRespID = String;
type CacheStatus = String;
type ResponseTime = String;
type QueryString = String;
type RespCodeAndSize = String;
type AgentCode = String;

const IDX_REQ_AND_RESP_ID: usize = 0;
const IDX_CACHE_STATUS: usize = 1;
#[derive(Debug)]
struct XIInfo {
    req_and_resp_id: Option<ReqAndRespID>,
    cache_status: Option<CacheStatus>,
    response_time: Option<ResponseTime>,
    query_string: Option<QueryString>,
    resp_code_and_size: Option<RespCodeAndSize>,
    agent_code: Option<AgentCode>,
}
impl XIInfo {
    fn parse(value: &String) -> Option<XIInfo> {
        let re = Regex::new(r"X-iinfo:\s*(.*)").unwrap();
        let captures = re.captures(value);
        if let Some(captures) = captures {
            let x_iinfo_value = captures.get(1).unwrap().as_str();
            let values: Vec<&str> = x_iinfo_value.split(' ').collect();
            let mut xi_info = XIInfo {
                req_and_resp_id: None,
                cache_status: None,
                response_time: None,
                query_string: None,
                resp_code_and_size: None,
                agent_code: None,
            };
            let mut x_iinfo_data_list = Vec::new();
            for (index, value) in values.iter().enumerate() {
                if index == IDX_REQ_AND_RESP_ID {
                    let req_and_resp_id = value.to_string();
                    xi_info.req_and_resp_id = Some(req_and_resp_id);
                } else if index == IDX_CACHE_STATUS {
                    let cache_status = value.to_string();
                    xi_info.cache_status = Some(cache_status);
                } else {
                    let value = value.to_string();
                    if value.eq("RT") {
                        x_iinfo_data_list.push(value);
                    } else if value.eq("q") {
                        xi_info.response_time = Some(x_iinfo_data_list.join(" "));
                        x_iinfo_data_list.clear();
                        x_iinfo_data_list.push(value);
                    } else if value.eq("r") {
                        xi_info.query_string = Some(x_iinfo_data_list.join(" "));
                        x_iinfo_data_list.clear();
                        x_iinfo_data_list.push(value);
                    } else if value.starts_with("U") {
                        xi_info.resp_code_and_size = Some(x_iinfo_data_list.join(" "));
                        let agent_code = value;
                        xi_info.agent_code = Some(agent_code);
                    } else {
                        x_iinfo_data_list.push(value);
                    }
                }
            }
            Some(xi_info)
        }else {
            None
        }
    }
    fn is_cache_hit(&self) -> bool {
        let _cache_status = self.cache_status.as_deref().unwrap();
        return true;
    }
}

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
    pretty_env_logger::init();

    let cli = Cli::parse();
    let url = cli.url;
    let tag = cli.tag.unwrap_or("img".to_string());
    let attr = cli.attr.unwrap_or("src".to_string());

    info!("open url: {}", &url);
    let client = Client::builder().use_rustls_tls();
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
                               "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15")
            .header("Accept","application/vnd.npm.install-v1+json; q=1.0, application/json; q=0.8, */*")
            .header("Content-Encoding","gzip").send().await;
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
                    let resource_url = format!(
                        "{}://{}{}",
                        url_info.scheme(),
                        url_info.host_str().unwrap(),
                        x
                    );
                    let start = Instant::now();
                    let resp = client.get(&resource_url).send().await;
                    if let Ok(resp) = resp {
                        if resp.status().is_success() {
                            info!("Success: {}", &resource_url);
                            let cache_info = resp.headers().get("x-iinfo");
                            let content_type = resp.headers().get("content-type");
                            /*
                            if let Some(cache_info) = cache_info {
                                let cache_status = cache_info.to_str().unwrap().chars().nth(9).unwrap();
                                match cache_status {
                                    'C' => {
                                        info!("cache_status: {} : {}", cache_status,"The resource is served from cache");
                                    }
                                    'V' => {
                                        info!("cache_status: {} : {}", cache_status,"The resource passed validation and is fresh.");
                                    }
                                    'N' => {
                                        info!("cache_status: {} : {}", cache_status,"The resource is not served from cache and was fetched directly from the backend.");
                                    }
                                    _ => {
                                        info!("cache_status: {}", cache_status);
                                    }
                                }
                            }*/
                            if let Some(cache_info) = cache_info {
                                let xi_info =
                                    XIInfo::parse(&cache_info.to_str().unwrap().to_string());
                                if let Some(xi_info) = xi_info {
                                    info!("x-iinfo: cache status {:#?}", xi_info.cache_status.unwrap_or("Unknown".to_string()));
                                }
                            }
                            if let Some(content_type) = content_type {
                                info!("content-type: {}", content_type.to_str().unwrap());
                            }
                        } else {
                            error!("Error: {}", &full_url);
                        }
                        let elapsed = start.elapsed();
                        let message = format!("Download use Elapsed time: {:.2?}\n=====", elapsed);
                        info!("{}", message);
                    }
                };
                futures.push(future);
            }
        });

    while let Some(_) = futures.next().await {}
}
