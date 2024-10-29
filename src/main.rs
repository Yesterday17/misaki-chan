#![feature(try_blocks)]
#![feature(try_trait_v2)]
#![feature(let_chains)]

mod bot;
mod config;
mod live;
mod parser;
mod user;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> anyhow::Result<()> {
    bot::start().await?;
    Ok(())
}
