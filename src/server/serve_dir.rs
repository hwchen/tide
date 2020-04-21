// Notes for making this module runtime agnostic
//
// Blocking steps needed:
// - Canonicalize path
// - Open file which returns impl AsyncRead
// - Get file metadata
//
// The last two are used to create the body. Packing them together would mean a fn would only need
// to return (len, AsyncRead) instead of needing to return a file.
//
// Canonicalize returns a PathBuf

use futures_io::AsyncRead;
use futures_util::io::BufReader;
use http_types::{Body, StatusCode};

use crate::{Endpoint, Request, Response, Result};

use std::io;
use std::path::{Path, PathBuf};

type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + 'a + Send>>;
pub struct ServeDir {
    prefix: String,
    dir: PathBuf,
    canonicalize: Box<dyn Fn(&Path) -> BoxFuture<'static, io::Result<PathBuf>> + Sync + Send>,
    file_open: Box<dyn Fn(&Path) -> BoxFuture<'static, io::Result<(usize, Box<dyn AsyncRead + Sync + Send + Unpin>)>> + Sync + Send>,
}

impl ServeDir {
    /// Create a new instance of `ServeDir`.
    pub(crate) fn new<CA, FO>(prefix: String, dir: PathBuf, canonicalize: CA, file_open: FO) -> Self
        where
            CA: Fn(&Path) -> BoxFuture<'static, io::Result<PathBuf>> + Sync + Send + 'static,
            FO: Fn(&Path) -> BoxFuture<'static, io::Result<(usize, Box<dyn AsyncRead + Sync + Send + Unpin>)>> + Sync + Send + 'static,
    {
        let canonicalize = Box::new(canonicalize);
        let file_open = Box::new(file_open);

        Self { prefix, dir, canonicalize, file_open }
    }
}

impl<State> Endpoint<State> for ServeDir {
    fn call<'a>(&'a self, req: Request<State>) -> BoxFuture<'a, Result<Response>> {
        let path = req.uri().path();
        let path = path.replacen(&self.prefix, "", 1);
        let path = path.trim_start_matches('/');
        let mut dir = self.dir.clone();
        for p in Path::new(path) {
            dir.push(&p);
        }
        log::info!("Requested file: {:?}", dir);

        Box::pin(async move {
            let canonicalize = &self.canonicalize;
            let (len, file) = match canonicalize(&dir).await {
                Err(_) => {
                    // This needs to return the same status code as the
                    // unauthorized case below to ensure we don't leak
                    // information of which files exist to adversaries.
                    log::warn!("File not found: {:?}", dir);
                    return Ok(Response::new(StatusCode::NotFound));
                }
                Ok(mut file_path) => {
                    // Verify this is a sub-path of the original dir.
                    let mut file_iter = (&mut file_path).iter();
                    if !dir.iter().all(|lhs| Some(lhs) == file_iter.next()) {
                        // This needs to return the same status code as the
                        // 404 case above to ensure we don't leak
                        // information about the local fs to adversaries.
                        log::warn!("Unauthorized attempt to read: {:?}", file_path);
                        return Ok(Response::new(StatusCode::NotFound));
                    }

                    // Open the file and send back the contents.
                    let file_open = &self.file_open;
                    match file_open(&file_path).await {
                        Ok((len, file)) => (len, file),
                        Err(_) => {
                            log::warn!("Could not open {:?}", file_path);
                            return Ok(Response::new(StatusCode::InternalServerError));
                        }
                    }
                }
            };

            let body = Body::from_reader(BufReader::new(file), Some(len));
            // TODO: fix related bug where async-h1 crashes on large files
            let mut res = Response::new(StatusCode::Ok);
            res.set_body(body);
            Ok(res)
        })
    }
}
