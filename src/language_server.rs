mod document;
mod format;
mod vfs;

use self::document::Document;
use self::format::format;
use self::vfs::VFS;
use crate::error::Error::LspIoError;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::runtime::Runtime;

use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use std::cmp::Ordering;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Debug)]
struct ServerBackend {
    client: Client,
    vfs: VFS,

    did_shutdown: Arc<RwLock<bool>>,
}

impl ServerBackend {
    fn new(client: Client, vfs: VFS, did_shutdown: Arc<RwLock<bool>>) -> ServerBackend {
        ServerBackend {
            client,
            vfs,
            did_shutdown,
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for ServerBackend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        let mut result = InitializeResult::default();
        result.capabilities.document_formatting_provider = Some(true);
        Ok(result)
    }
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.vfs
            .create_document(&params.text_document.uri, &params.text_document.text);
    }
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let doc_version = match self
            .vfs
            .with_document(&params.text_document.uri, Document::version)
        {
            None => {
                // Document is not currently opened, so we can't update changes to it.
                // Log warning to the client and do nothing.

                let error_message = format!(
                    "Received didChange event for unopened document.\n\tDocument: {}",
                    params.text_document.uri.path(),
                );
                self.client
                    .log_message(MessageType::Warning, error_message)
                    .await;

                return;
            }
            Some(ver) => ver,
        };

        match params.text_document.version {
            None => {
                // This should not happen!  While the field is nullable, null is only valid
                // when sent from server to client in certain situations.
                // We may be able to recover and continue, but it risks corrupting the document
                // in question, so it is safest to just pass the error to the client and
                // to panic.

                let error_message = format!(
                    "Received version null in didChange notification.\n\tDocument: {}",
                    params.text_document.uri.path()
                );
                self.client
                    .log_message(MessageType::Error, &error_message)
                    .await;
                panic!(error_message);
            }
            Some(version) => {
                match doc_version.cmp(&version) {
                    Ordering::Equal => {
                        self.vfs.modify_document(&params.text_document.uri, |doc| {
                            doc.apply_content_changes(&params.content_changes);
                        });
                    }
                    Ordering::Less => {
                        // We are being asked to operate on a version of the document that we
                        // do not have.  All we can do is error and panic.

                        let error_message = format!(
                            "Text synchronization failed.\n\tDocument: {}\n\tServer version: {}\n\tClient version:{}",
                            params.text_document.uri.path(),
                            doc_version,
                            version,
                        );
                        self.client
                            .log_message(MessageType::Error, &error_message)
                            .await;
                        panic!(error_message);
                    }
                    Ordering::Greater => {
                        // We have a newer version than the one being sent, so ignore the changes.

                        let log_message = format!(
                            "Skipping didChange - version on server newer.\n\tDocument: {}\n\tServer version: {}\n\tClient version:{}",
                            params.text_document.uri.path(),
                            doc_version,
                            version,
                        );
                        self.client
                            .log_message(MessageType::Info, log_message)
                            .await;
                    }
                }
            }
        };
    }
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.vfs.evict_document(&params.text_document.uri);
    }
    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let doc_uri = params.text_document.uri;
        let doc_contents = match self.vfs.get_document_contents(&doc_uri) {
            Ok(contents) => contents,
            Err(io_error) => {
                return Err(Error {
                    code: ErrorCode::InternalError,
                    message: io_error.to_string(),
                    data: None,
                })
            }
        };

        match format(doc_contents) {
            Ok(x) => Ok(Some(x)),
            Err(s) => Err(Error {
                code: ErrorCode::ParseError,
                message: s,
                data: None,
            }),
        }
    }
    async fn shutdown(&self) -> Result<()> {
        if let Ok(ref mut did_shutdown_ref) = self.did_shutdown.try_write() {
            **did_shutdown_ref = true;
            Ok(())
        } else {
            Err(Error {
                code: ErrorCode::InternalError,
                message: "Failed to lock did_shutdown_ref for writing".to_string(),
                data: None,
            })
        }
    }
}

// Runs the language server with the given input and output streams.
// Returns true if the server shutdown safely before exiting, otherwise false.
fn run_server<I, O>(stdin: I, stdout: O) -> std::io::Result<bool>
where
    I: AsyncRead + Unpin,
    O: AsyncWrite,
{
    let mut rt = Runtime::new().unwrap();

    let did_shutdown = Arc::new(RwLock::new(false));

    let vfs = VFS::new()?;

    let (service, messages) =
        LspService::new(|client| ServerBackend::new(client, vfs, did_shutdown.clone()));

    rt.block_on(async {
        Server::new(stdin, stdout)
            .interleave(messages)
            .serve(service)
            .await;
        if let Ok(did_shutdown_value) = did_shutdown.read() {
            Ok(*did_shutdown_value)
        } else {
            // If read is not Ok, the lock is poisoned - writer panicked
            // while the cell was locked for writing. We have to assume
            // in that case that the shutdown failed.

            Ok(false)
        }
    })
}

pub fn command() -> std::result::Result<i32, crate::error::Error> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let shutdown_before_exiting = match run_server(stdin, stdout) {
        Ok(b) => b,
        Err(err) => return Err(LspIoError { err: err.kind() }),
    };

    if shutdown_before_exiting {
        Ok(0)
    } else {
        Ok(1)
    }
}
