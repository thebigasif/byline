//! The Model Context Protocol as an agent speaks it: one JSON-RPC message
//! per line on stdin, one answer per line on stdout (KH-API-5).

use khalid_mcp::{BoxFuture, ServeError, Server, Tool, ToolResult, Tools};
use serde_json::{Map, Value, json};

/// One tool, `echo`, that answers with its `text` argument, or fails when
/// the text is empty.
#[derive(Debug)]
struct Echo(Vec<Tool>);

impl Echo {
    fn new() -> Self {
        let schema = json!({
            "type": "object",
            "properties": { "text": { "type": "string" } },
            "required": ["text"]
        });
        Self(vec![Tool::new("echo", "Answers with the text.", schema)])
    }
}

impl Tools for Echo {
    fn list(&self) -> &[Tool] {
        &self.0
    }

    fn call<'a>(
        &'a self,
        name: &'a str,
        arguments: Map<String, Value>,
    ) -> BoxFuture<'a, Option<ToolResult>> {
        let text = arguments.get("text").and_then(Value::as_str);
        let answer = match (name, text) {
            ("echo", Some("")) => Some(ToolResult::failed("The text is empty.")),
            ("echo", Some(text)) => Some(ToolResult::done(text)),
            ("echo", None) => Some(ToolResult::failed("The text is missing.")),
            _ => None,
        };
        Box::pin(std::future::ready(answer))
    }
}

/// Serves `lines`, each one message, and returns every answer.
#[cfg(test)]
async fn session(lines: &[Value]) -> Vec<Value> {
    let mut input = String::new();
    for line in lines {
        input.push_str(&line.to_string());
        input.push('\n');
    }
    answers(input.as_bytes()).await
}

#[cfg(test)]
async fn answers(input: &[u8]) -> Vec<Value> {
    let echo = Echo::new();
    let server = Server::new("khalid", "1.2.3", "Use echo to test.", &echo);
    let mut output = Vec::new();
    server.serve(input, &mut output).await.unwrap();
    let text = String::from_utf8(output).unwrap();
    text.lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[cfg(test)]
fn request(id: u64, method: &str, params: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
}

#[cfg(test)]
fn initialize(version: &str) -> Value {
    request(
        1,
        "initialize",
        &json!({
            "protocolVersion": version,
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "0" }
        }),
    )
}

#[tokio::test]
async fn initialize_agrees_the_version_asked_for_and_offers_tools() {
    let answers = session(&[initialize("2025-06-18")]).await;
    assert_eq!(
        answers,
        [json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2025-06-18",
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "khalid", "version": "1.2.3" },
                "instructions": "Use echo to test."
            }
        })]
    );
}

#[tokio::test]
async fn an_unknown_version_is_answered_with_the_newest_one() {
    let answers = session(&[initialize("1999-01-01")]).await;
    assert_eq!(answers[0]["result"]["protocolVersion"], "2025-11-25");
}

#[tokio::test]
async fn notifications_get_no_answer_and_ping_gets_an_empty_one() {
    let initialized = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
    let answers = session(&[initialized, request(7, "ping", &json!({}))]).await;
    assert_eq!(
        answers,
        [json!({ "jsonrpc": "2.0", "id": 7, "result": {} })]
    );
}

#[tokio::test]
async fn tools_list_names_each_tool_with_its_input_schema() {
    let answers = session(&[request(2, "tools/list", &json!({}))]).await;
    let tools = &answers[0]["result"]["tools"];
    assert_eq!(tools[0]["name"], "echo");
    assert_eq!(tools[0]["description"], "Answers with the text.");
    assert_eq!(tools[0]["inputSchema"]["required"], json!(["text"]));
    assert_eq!(tools.as_array().map(Vec::len), Some(1));
}

#[tokio::test]
async fn a_tool_call_answers_with_text() {
    let call = json!({ "name": "echo", "arguments": { "text": "Hallo" } });
    let answers = session(&[request(3, "tools/call", &call)]).await;
    assert_eq!(
        answers[0]["result"],
        json!({ "content": [{ "type": "text", "text": "Hallo" }], "isError": false })
    );
}

#[tokio::test]
async fn a_tool_that_fails_says_so_in_its_result() {
    let call = json!({ "name": "echo", "arguments": { "text": "" } });
    let answers = session(&[request(4, "tools/call", &call)]).await;
    assert_eq!(answers[0]["result"]["isError"], true);
    assert_eq!(
        answers[0]["result"]["content"][0]["text"],
        "The text is empty."
    );
}

#[tokio::test]
async fn a_call_without_arguments_reaches_the_tool_with_none() {
    let answers = session(&[request(5, "tools/call", &json!({ "name": "echo" }))]).await;
    assert_eq!(
        answers[0]["result"]["content"][0]["text"],
        "The text is missing."
    );
}

#[tokio::test]
async fn an_unknown_tool_is_invalid_params() {
    let call = json!({ "name": "delete_everything", "arguments": {} });
    let answers = session(&[request(6, "tools/call", &call)]).await;
    assert_eq!(answers[0]["error"]["code"], -32602);
    assert_eq!(answers[0]["id"], 6);
    let message = answers[0]["error"]["message"].as_str().unwrap();
    assert!(message.contains("delete_everything"), "{message}");
}

#[tokio::test]
async fn an_unknown_method_is_method_not_found() {
    let answers = session(&[request(8, "resources/list", &json!({}))]).await;
    assert_eq!(answers[0]["error"]["code"], -32601);
}

#[tokio::test]
async fn a_line_that_is_not_json_is_a_parse_error_and_the_session_goes_on() {
    let answers = answers(b"{not json\n{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"ping\"}\n").await;
    assert_eq!(answers[0]["error"]["code"], -32700);
    assert_eq!(answers[0]["id"], Value::Null);
    assert_eq!(answers[1]["result"], json!({}));
}

#[tokio::test]
async fn a_message_that_is_not_a_request_is_invalid() {
    let batch = json!([request(10, "ping", &json!({}))]);
    let wrong_version = json!({ "jsonrpc": "1.0", "id": 11, "method": "ping" });
    let answers = session(&[batch, wrong_version]).await;
    assert_eq!(answers[0]["error"]["code"], -32600);
    assert_eq!(answers[1]["error"]["code"], -32600);
    assert_eq!(answers[1]["id"], 11);
}

#[tokio::test]
async fn a_message_longer_than_the_limit_ends_the_session() {
    let echo = Echo::new();
    let server = Server::new("khalid", "1.2.3", "", &echo);
    let input = vec![b'x'; khalid_mcp::MAX_MESSAGE + 1];
    let mut output = Vec::new();
    let error = server.serve(&input[..], &mut output).await.unwrap_err();
    assert!(matches!(error, ServeError::TooLong), "{error:?}");
    assert_eq!(output, b"");
}
