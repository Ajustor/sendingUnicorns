use std::{collections::HashMap, str::FromStr};

use serde_json::Value;
use tauri::http::{HeaderMap, HeaderName, HeaderValue};
use tauri_plugin_http::reqwest::{self, Error, RequestBuilder};

use super::structs::RequestOptions;

pub async fn call(
    method: String,
    url: String,
    request_options: RequestOptions,
) -> Result<String, Error> {
    let mut client_builder: reqwest::ClientBuilder =
        reqwest::Client::builder().danger_accept_invalid_certs(true);

    client_builder = add_headers_to_client(client_builder, request_options.headers);

    let request = client_builder.build().unwrap();

    if method == "get" {
        return get(request, url, request_options.body).await;
    }

    if method == "post" {
        return post(request, url, request_options.body).await;
    }

    return get(request, url, request_options.body).await;
}

async fn get(
    request: reqwest::Client,
    url: String,
    body: Option<HashMap<String, Value>>,
) -> Result<String, Error> {
    let resp = add_body_to_request(request.get(url), body).send().await;

    if resp.is_err() {
        return Err(resp.err().unwrap());
    }

    return Ok(resp.unwrap().text().await.unwrap());
}

fn add_headers_to_client(
    client_builder: reqwest::ClientBuilder,
    request_headers: Option<Vec<[String; 2]>>,
) -> reqwest::ClientBuilder {
    let mut headers = HeaderMap::new();

    match request_headers {
        None => {}
        Some(some_request_headers) => {
            for header in some_request_headers.clone() {
                let header_name = HeaderName::from_str(header[0].as_str()).unwrap();
                let header_value = HeaderValue::from_str(header[1].as_str()).unwrap();
                headers.insert(header_name, header_value);
            }
        }
    }

    return client_builder.default_headers(headers);
}

fn add_body_to_request(
    request_builder: RequestBuilder,
    body: Option<HashMap<String, Value>>,
) -> RequestBuilder {
    match body {
        None => {
            return request_builder;
        }
        Some(json_data) => {
            if json_data.keys().len() != 0 {
                println!("Adding body to request {:?}", json_data);
                return request_builder.json(&json_data);
            }
            return request_builder;
        }
    };
}

async fn post(
    client: reqwest::Client,
    url: String,
    body: Option<HashMap<String, Value>>,
) -> Result<String, Error> {
    let request_builder = add_body_to_request(client.post(url), body);

    return request_builder.send().await.unwrap().text().await;
}
