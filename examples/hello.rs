use async_std::net::TcpListener;
use async_std::task;
use futures_util::stream::StreamExt;
use std::sync::Arc;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = tide::new();
        app.at("/").get(|_| async move { Ok("Hello, world!") });

        let listener = TcpListener::bind("127.0.0.1:8080").await?;

        let addr = format!("http://{}", listener.local_addr()?);
        log::info!("Server is listening on: {}", addr);

        let server = Arc::new(app);

        while let Some(stream) = listener.incoming().next().await {
            let stream = stream?;
            task::spawn(tide::accept(addr.clone(), server.clone(), stream));
        }

        Ok(())
}
