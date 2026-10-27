//! The stdio transport and the methods a Khalid server answers.

use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::tool::Tools;

/// The longest message read, in bytes. A post's whole markdown body fits
/// many times over; a longer line is a client fault, and reading on would
/// let it fill memory.
pub const MAX_MESSAGE: usize = 8 * 1024 * 1024;

/// The protocol versions this server speaks, newest first. Their tool
/// methods are the same; a client asking for another gets the newest.
const VERSIONS: [&str; 4] = [NEWEST, "2025-06-18", "2025-03-26", "2024-11-05"];

/// The newest version this server speaks.
const NEWEST: &str = "2025-11-25";

/// JSON-RPC error codes (JSON-RPC 2.0 section 5.1).
const PARSE_ERROR: i64 = -32_700;
const INVALID_REQUEST: i64 = -32_600;
const METHOD_NOT_FOUND: i64 = -32_601;

/// A server that answers on one pair of streams.
#[derive(Debug, Clone, Copy)]
pub struct Server<'t> {
    name: &'t str,
    version: &'t str,
    instructions: &'t str,
    tools: &'t dyn Tools,
}

/// Why a session ended before its input did.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ServeError {
    /// A message was longer than [`MAX_MESSAGE`].
    #[error("a message was longer than {MAX_MESSAGE} bytes")]
    TooLong,
    /// The input could not be read.
    #[error("the messages could not be read")]
    Read(#[source] std::io::Error),
    /// An answer could not be written.
    #[error("an answer could not be written")]
    Write(#[source] std::io::Error),
}

/// A request or notification, before its method is known.
#[derive(Deserialize)]
struct Request {
    jsonrpc: String,
    /// Absent on a notification, which gets no answer.
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

impl<'t> Server<'t> {
    /// A server named `name` at `version`, giving agents `instructions`
    /// when they connect and offering `tools`.
    pub fn new(
        name: &'t str,
        version: &'t str,
        instructions: &'t str,
        tools: &'t dyn Tools,
    ) -> Self {
        Self {
            name,
            version,
            instructions,
            tools,
        }
    }

    /// Answers each message on `input` with one line on `output`, until
    /// the input ends. Calls run one at a time, in the order they came.
    ///
    /// # Errors
    ///
    /// Returns [`ServeError::TooLong`] for a message over [`MAX_MESSAGE`]
    /// bytes, and [`ServeError::Read`] or [`ServeError::Write`] when a
    /// stream fails. A message that is not valid JSON-RPC gets an error
    /// answer and the session goes on.
    pub async fn serve(
        &self,
        mut input: impl AsyncBufRead + Unpin,
        mut output: impl AsyncWrite + Unpin,
    ) -> Result<(), ServeError> {
        let mut line = Vec::new();
        loop {
            line.clear();
            let limit = u64::try_from(MAX_MESSAGE).unwrap_or(u64::MAX);
            let read = (&mut input)
                .take(limit)
                .read_until(b'\n', &mut line)
                .await
                .map_err(ServeError::Read)?;
            if read == 0 {
                return Ok(());
            }
            if read == MAX_MESSAGE && !line.ends_with(b"\n") {
                return Err(ServeError::TooLong);
            }
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            if let Some(answer) = self.answer(&line) {
                let mut bytes = answer.to_string().into_bytes();
                bytes.push(b'\n');
                output.write_all(&bytes).await.map_err(ServeError::Write)?;
                output.flush().await.map_err(ServeError::Write)?;
            }
        }
    }

    /// The answer to one message, or `None` for a notification.
    fn answer(&self, line: &[u8]) -> Option<Value> {
        let Ok(message) = serde_json::from_slice::<Value>(line) else {
            return Some(failure(
                &Value::Null,
                PARSE_ERROR,
                "The message is not JSON.",
            ));
        };
        let id = message.get("id").cloned().unwrap_or(Value::Null);
        let request = match serde_json::from_value::<Request>(message) {
            Ok(request) if request.jsonrpc == "2.0" => request,
            _ => {
                let reason = "The message is not a JSON-RPC 2.0 request.";
                return Some(failure(&id, INVALID_REQUEST, reason));
            }
        };
        let id = request.id?;
        let answer = match request.method.as_str() {
            "initialize" => Ok(self.initialize(&request.params)),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": self.tools.list() })),
            method => Err((METHOD_NOT_FOUND, format!("There is no method {method}."))),
        };
        Some(match answer {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, reason)) => failure(&id, code, &reason),
        })
    }

    /// Agrees the version the client asked for when this server speaks it,
    /// or else offers the newest.
    fn initialize(&self, params: &Value) -> Value {
        let asked = params.get("protocolVersion").and_then(Value::as_str);
        let version = VERSIONS
            .into_iter()
            .find(|version| Some(*version) == asked)
            .unwrap_or(NEWEST);
        json!({
            "protocolVersion": version,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": self.name, "version": self.version },
            "instructions": self.instructions,
        })
    }
}

/// A JSON-RPC error answer to the request `id`.
fn failure(id: &Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
}
