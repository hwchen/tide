use async_std::fs::File;
use async_std::net::TcpListener;
use async_std::task;
use futures_core::future::BoxFuture;
use futures_io::AsyncRead;
use futures_util::future::FutureExt;
use futures_util::stream::StreamExt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    femme::start(log::LevelFilter::Info).unwrap();

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

fn file_open(file_path: &'static Path) -> BoxFuture<'static, io::Result<(usize, Box<dyn AsyncRead + Sync + Send + Unpin>)>> {
    let len_file = File::open(&file_path)
        .then(|file| {
            let file = file.unwrap();
            file.metadata()
                .map(|metadata| {
                    Ok((
                        metadata.unwrap().len() as usize,
                        Box::new(file) as Box<dyn AsyncRead + Sync + Send + Unpin>
                    ))
                })
        });

    let boxed = Box::new(len_file) as Box<dyn std::future::Future<Output=io::Result<(usize, Box<dyn AsyncRead + Sync + Send + Unpin + 'static>)>> + Send>;

    unsafe {
        use std::pin::Pin;
        Pin::new_unchecked(boxed)
    }
    //Box::pin(futures_util::future::ok(len_file))
    //Box::pin(File::open(file_path).map(|f|f.unwrap())) as Box<dyn AsyncRead + Sync + Send + Unpin>)))
}

fn canonicalize(file_path: &Path) -> BoxFuture<'static, io::Result<PathBuf>> {
    //Box::pin(
    //    futures_util::future::ok(
    //        fs::canonicalize(file_path).and_then(|path| PathBuf::from(path.into_os_string())))
    //)
    Box::pin(futures_util::future::ok(PathBuf::from("a_path")))
}

// Still failed. Probably need to not pass in bare closures, it gets too complicated. See how
// actix-web turns a closure into a Service: https://github.com/actix/actix-web/blob/master/src/route.rs#L226
// But that's a lot of type programming. Probably best to just implement it myself each time I need
// it. Just use the current one as an example.
