mod support;
use support::*;

use std::time::Duration;

#[tokio::test]
async fn request_timeout() {
    let _ = env_logger::try_init();

    let server = server::http(move |_req| {
        async {
            // delay returning the response
            tokio::time::delay_for(Duration::from_secs(2)).await;
            http::Response::default()
        }
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap();

    let url = format!("http://{}/slow", server.addr());

    let res = client.get(&url).send().await;

    let err = res.unwrap_err();

    assert!(err.is_timeout());
    assert_eq!(err.url().map(|u| u.as_str()), Some(url.as_str()));
}

#[tokio::test]
async fn response_timeout() {
    let _ = env_logger::try_init();

    let server = server::http(move |_req| {
        async {
            // immediate response, but delayed body
            let body = hyper::Body::wrap_stream(futures_util::stream::once(async {
                tokio::time::delay_for(Duration::from_secs(2)).await;
                Ok::<_, std::convert::Infallible>("Hello")
            }));

            http::Response::new(body)
        }
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap();

    let url = format!("http://{}/slow", server.addr());
    let res = client.get(&url).send().await.expect("Failed to get");
    let body = res.text().await;

    let err = body.unwrap_err();

    assert!(err.is_timeout());
}

/// Tests that internal client future cancels when the oneshot channel
/// is canceled.
#[cfg(feature = "blocking")]
#[test]
fn timeout_closes_connection() {
    let _ = env_logger::try_init();

    // Make Client drop *after* the Server, so the background doesn't
    // close too early.
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap();

    let server = server::http(move |_req| {
        async {
            // delay returning the response
            tokio::time::delay_for(Duration::from_secs(2)).await;
            http::Response::default()
        }
    });

    let url = format!("http://{}/closes", server.addr());
    let err = client.get(&url).send().unwrap_err();

    assert!(err.is_timeout());
    assert_eq!(err.url().map(|u| u.as_str()), Some(url.as_str()));
}

#[cfg(feature = "blocking")]
#[test]
fn write_timeout_large_body() {
    let _ = env_logger::try_init();
    let body = vec![b'x'; 20_000];
    let len = 8192;

    // Make Client drop *after* the Server, so the background doesn't
    // close too early.
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap();

    let server = server::http(move |_req| {
        async {
            // delay returning the response
            tokio::time::delay_for(Duration::from_secs(2)).await;
            http::Response::default()
        }
    });

    let cursor = std::io::Cursor::new(body);
    let url = format!("http://{}/write-timeout", server.addr());
    let err = client
        .post(&url)
        .body(reqwest::blocking::Body::sized(cursor, len as u64))
        .send()
        .unwrap_err();

    assert!(err.is_timeout());
    assert_eq!(err.url().map(|u| u.as_str()), Some(url.as_str()));
}
