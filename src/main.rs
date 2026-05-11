use std::{net::SocketAddr, sync::Arc};

use rust_api_example::{TagList, app};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = TcpListener::bind(address).await?;
    let tag_list = Arc::new(TagList::new());

    println!("listening on http://{address}");

    axum::serve(listener, app(tag_list)).await?;

    Ok(())
}
