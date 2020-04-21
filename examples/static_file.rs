use async_std::fs::{self, File};
use async_std::net::TcpListener;
use async_std::task;
use futures_core::future::{BoxFuture, Future};
use futures_io::AsyncRead;
use futures_util::future::{FutureExt, TryFutureExt};
use futures_util::stream::StreamExt;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    femme::start(log::LevelFilter::Info).unwrap();

    //let canonicalize = |file_path| async {
    //    Ok(fs::canonicalize(file_path).await?.into_os_string().into())
    //}

    let mut app = tide::new();
    app.at("/").get(|_| async move { Ok("visit /src/*") });
    app.at("/src").serve_dir("src/", canonicalize, file_open)?;

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

fn file_open(file_path: &Path) -> BoxFuture<'static, io::Result<(usize, Box<dyn AsyncRead + Sync + Send + Unpin>)>>
{
//    let len_file = File::open(&file_path)
//        .and_then(|file| {
//            file.metadata()
//                .and_then(|metadata| {
//                    (metadata.len() as usize, Box::new(file))
//                })
//        });
//
//
    //Box::pin(futures_util::future::ready(len_file))
    Box::pin(futures_util::future::ok((1, Box::new(File::open(file_path).map(|f|f.unwrap())) as Box<AsyncRead + Sync + Send + Unpin>)))
}

fn canonicalize(file_path: &Path) -> BoxFuture<'static, io::Result<PathBuf>> {
    //Box::pin(
    //    futures_util::future::ok(
    //        fs::canonicalize(file_path).and_then(|path| PathBuf::from(path.into_os_string())))
    //)
    Box::pin(futures_util::future::ok(PathBuf::from("a_path")))
}

