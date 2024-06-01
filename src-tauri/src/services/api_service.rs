use tauri_plugin_http::reqwest;

pub async fn call(method: String, url: String) -> String {
    let request = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    if method == "get" {
        return get(request, url).await;
    }

    if method == "post" {
        return post(request, url).await;
    }

    return get(request, url).await;
}

async fn get(request: reqwest::Client, url: String) -> String {
    let resp = request.get(url).send().await.unwrap().text().await.unwrap();

    return resp;
}

async fn post(request: reqwest::Client, url: String) -> String {
    let resp = request
        .post(url)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    return resp;
}
