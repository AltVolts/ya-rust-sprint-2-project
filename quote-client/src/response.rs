const SERVER_STREAM_RESPONSE: &str = "OK";

pub enum ServerTcpResponse {
    OK,
    ErrorResponse(String),
}

pub fn is_response_ok(response: String) -> ServerTcpResponse {
    let input = response.trim();
    match input {
        SERVER_STREAM_RESPONSE => ServerTcpResponse::OK,
        input => ServerTcpResponse::ErrorResponse(input.to_string()),
    }
}
