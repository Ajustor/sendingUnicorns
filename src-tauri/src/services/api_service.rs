use std::{collections::HashMap, str::FromStr};

use serde_json::Value;
use tauri::{
    http::{HeaderMap, HeaderName, HeaderValue},
    Url,
};
use tauri_plugin_http::reqwest::{self, Error, RequestBuilder};

use super::structs::{BodyTypes, BodyTypesEnum, DynamicValue, RequestParams};

pub async fn call(
    method: String,
    url: String,
    request_options: RequestParams,
    body_type: BodyTypesEnum,
) -> Result<String, Error> {
    let mut client_builder: reqwest::ClientBuilder =
        reqwest::Client::builder().danger_accept_invalid_certs(true);

    client_builder = add_headers_to_client(client_builder, request_options.headers);

    if let Ok(Some(proxy_config)) = proxy_cfg::get_proxy_config() {
        if let Some(proxy_address) = proxy_config.get_proxy_for_url(&Url::parse(&url).unwrap()) {
            client_builder = client_builder.proxy(reqwest::Proxy::http(proxy_address)?);
        }
    }

    let request = client_builder.build().unwrap();

    if method == "get" {
        return get(request, url, request_options.body, body_type).await;
    }

    if method == "post" {
        return post(request, url, request_options.body, body_type).await;
    }

    if method == "patch" {
        return patch(request, url, request_options.body, body_type).await;
    }

    if method == "put" {
        return put(request, url, request_options.body, body_type).await;
    }

    if method == "delete" {
        return delete(request, url, request_options.body, body_type).await;
    }

    return get(request, url, request_options.body, body_type).await;
}

async fn get(
    request: reqwest::Client,
    url: String,
    body: Option<BodyTypes>,
    body_type: BodyTypesEnum,
) -> Result<String, Error> {
    let resp = add_body_to_request(request.get(url), body, body_type)
        .send()
        .await;

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
    body: Option<BodyTypes>,
    body_type: BodyTypesEnum,
) -> RequestBuilder {
    println!("Sending request with body {:?}", body);

    match body {
        None => {
            println!("Sending request with not body {:?}", body);
            return request_builder;
        }
        Some(existing_body) => {
            match body_type {
                BodyTypesEnum::FormData => {
                    let form_data = existing_body
                        .form_data
                        .into_iter() // chunks_exact returns an iterator of slices
                        .filter_map(|chunk| chunk.1.is_active.then(|| (chunk.0, chunk.1.value))) // map slices to tuples
                        .collect::<HashMap<String, DynamicValue>>();

                    return request_builder.form(&form_data);
                }
                BodyTypesEnum::Json => {
                    let parsed_body: HashMap<String, Value> =
                        serde_json::from_str(&existing_body.json)
                            .expect("JSON was not well-formatted");
                    println!("Sending request with body parsed as {:?}", parsed_body);

                    if parsed_body.keys().len() != 0 {
                        return request_builder.json(&parsed_body);
                    }
                }
            }

            return request_builder;
        }
    };
}

async fn post(
    client: reqwest::Client,
    url: String,
    body: Option<BodyTypes>,
    body_type: BodyTypesEnum,
) -> Result<String, Error> {
    let resp = add_body_to_request(client.post(url), body, body_type)
        .send()
        .await;

    if resp.is_err() {
        return Err(resp.err().unwrap());
    }

    return Ok(resp.unwrap().text().await.unwrap());
}

async fn patch(
    client: reqwest::Client,
    url: String,
    body: Option<BodyTypes>,
    body_type: BodyTypesEnum,
) -> Result<String, Error> {
    let resp = add_body_to_request(client.patch(url), body, body_type)
        .send()
        .await;

    if resp.is_err() {
        return Err(resp.err().unwrap());
    }

    return Ok(resp.unwrap().text().await.unwrap());
}

async fn put(
    client: reqwest::Client,
    url: String,
    body: Option<BodyTypes>,
    body_type: BodyTypesEnum,
) -> Result<String, Error> {
    let resp = add_body_to_request(client.put(url), body, body_type)
        .send()
        .await;

    if resp.is_err() {
        return Err(resp.err().unwrap());
    }

    return Ok(resp.unwrap().text().await.unwrap());
}

async fn delete(
    client: reqwest::Client,
    url: String,
    body: Option<BodyTypes>,
    body_type: BodyTypesEnum,
) -> Result<String, Error> {
    let resp = add_body_to_request(client.delete(url), body, body_type)
        .send()
        .await;

    if resp.is_err() {
        return Err(resp.err().unwrap());
    }

    return Ok(resp.unwrap().text().await.unwrap());
}
