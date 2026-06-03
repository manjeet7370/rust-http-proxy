use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub raw: String,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub fn parse_request(raw: &str) -> Result<HttpRequest, String> {
    let mut lines = raw.lines();

    let req_line = lines.next().ok_or("Empty request")?;
    let tokens: Vec<&str> = req_line.split_whitespace().collect();

    if tokens.len() < 3 {
        return Err(format!("Invalid request line: {}", req_line));
    }

    let method = tokens[0].to_string();
    let path = tokens[1].to_string();
    let version = tokens[2].to_string();

    let mut headers = HashMap::new();
    let mut body = Vec::new();
    let mut in_body = false;

    for line in lines {
        if line.trim().is_empty() {
            in_body = true;
            continue;
        }

        if in_body {
            body.push(line);
            continue;
        }

        if let Some((k, v)) = line.split_once(':') {
            headers.insert(
                k.trim().to_ascii_lowercase(),
                v.trim().to_string(),
            );
        }
    }

    let body = if body.is_empty() {
        None
    } else {
        Some(body.join("\n"))
    };

    Ok(HttpRequest {
        method,
        path,
        version,
        headers,
        body,
        raw: raw.to_string(),
    })
}

pub fn parse_response(raw: &str) -> Result<HttpResponse, String> {
    let mut lines = raw.lines();

    let status_line = lines.next().ok_or("Empty response")?;
    let parts: Vec<&str> = status_line.split_whitespace().collect();

    if parts.len() < 2 {
        return Err(format!("Invalid status line: {}", status_line));
    }

    let status_code = parts[1]
        .parse::<u16>()
        .map_err(|_| "Invalid status code".to_string())?;

    let status_text = if parts.len() > 2 {
        parts[2..].join(" ")
    } else {
        String::new()
    };

    let mut headers = HashMap::new();
    let mut body_lines = Vec::new();
    let mut reached_body = false;

    for line in lines {
        if line.trim().is_empty() {
            reached_body = true;
            continue;
        }

        if reached_body {
            body_lines.push(line);
            continue;
        }

        if let Some((k, v)) = line.split_once(':') {
            headers.insert(
                k.trim().to_ascii_lowercase(),
                v.trim().to_string(),
            );
        }
    }

    Ok(HttpResponse {
        status_code,
        status_text,
        headers,
        body: body_lines.join("\n"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_get() {
        let raw =
            "GET /home HTTP/1.1\r\nHost: example.com\r\nUser-Agent: Test\r\n\r\n";

        let req = parse_request(raw).unwrap();

        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/home");
        assert_eq!(req.version, "HTTP/1.1");
        assert_eq!(req.headers.get("host").unwrap(), "example.com");
    }

    #[test]
    fn parse_post_with_body() {
        let raw = "POST /login HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\r\n{\"user\":\"manjeet\"}";

        let req = parse_request(raw).unwrap();

        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/login");
        assert!(req.body.is_some());
        assert_eq!(
            req.body.unwrap(),
            "{\"user\":\"manjeet\"}"
        );
    }

    #[test]
    fn invalid_request_should_fail() {
        let raw = "HELLO_WORLD";

        assert!(parse_request(raw).is_err());
    }

    #[test]
    fn parse_response_ok() {
        let raw = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello World";

        let res = parse_response(raw).unwrap();

        assert_eq!(res.status_code, 200);
        assert_eq!(res.status_text, "OK");
        assert_eq!(
            res.headers.get("content-type").unwrap(),
            "text/plain"
        );
        assert_eq!(res.body, "Hello World");
    }

    #[test]
    fn invalid_response_should_fail() {
        let raw = "HTTP/1.1";

        assert!(parse_response(raw).is_err());
    }
}