#[tokio::main]
async fn main() -> Result<(),std::io::Error>{
    pretty_env_logger::init();
    println!("Hello, world!");
    Ok(())
}
