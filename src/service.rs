pub async fn download_16m(link: &str, cookie: Option<String>, authorization: Option<String>) -> Option<String> {
  let client = reqwest::Client::new();

  tracing::debug!("Start download {}", link);

  let mut request = client
    .get(link)
    .header("Range", "bytes=0-16777215");

  if let Some(cookie) = cookie {
    request = request.header("Cookie", cookie);
  }

  if let Some(authorization) = authorization {
    request = request.header("Authorization", authorization );
  }

  let resp = request.send().await;

  match resp {
    Ok(resp) => {
      tracing::debug!("Successfully download {}", link);

      let body = resp.bytes().await.unwrap();
      let md5 = md5::compute(body);
      let md5_str = format!("{:x}", md5);

      tracing::debug!("Successfully calculating md5 {}", link);

      return Some(md5_str);
    }

    Err(e) => {
      tracing::error!("download failed for {}: {}", link, e);
      return None;
    }
  }
}
