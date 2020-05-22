pub mod chat;

use bytes::buf::BufExt as _;
use hyper::client::*;
use hyper::{Body, Request, Uri};
use rsb_derive::Builder;
use url::Url;

#[derive(Debug, PartialEq, Clone, Builder)]
pub struct SlackApiToken {
    value: String,
    workspace_id: Option<String>,
    scope: Option<String>,
}

#[derive(Debug)]
pub struct SlackClient {
    connector: Client<HttpConnector>,
}

#[derive(Debug)]
pub struct SlackClientSession<'a> {
    client: &'a SlackClient,
    token: SlackApiToken,
}

pub type ClientResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

impl SlackClient {
    const SLACK_API_URI_STR: &'static str = "https://slack.com/api";

    fn create_method_uri_path(method_relative_uri: &str) -> String {
        format!("{}/{}", SlackClient::SLACK_API_URI_STR, method_relative_uri)
    }

    fn create_url(url_str: &String) -> Uri {
        url_str.parse().unwrap()
    }

    fn create_url_with_params<PT, TS>(url_str: &String, params: PT) -> Uri
    where
        PT: std::iter::IntoIterator<Item = (TS, Option<TS>)>,
        TS: std::string::ToString,
    {
        let url_query_params: Vec<(String, String)> = params
            .into_iter()
            .map(|(k, vo)| vo.map(|v| (k.to_string(), v.to_string())))
            .flatten()
            .collect();

        Url::parse_with_params(url_str.as_str(), url_query_params)
            .unwrap()
            .as_str()
            .parse()
            .unwrap()
    }

    pub fn new() -> Self {
        SlackClient {
            connector: Client::new(),
        }
    }

    pub async fn send_webapi_request<RS>(&self, request: Request<Body>) -> ClientResult<RS>
    where
        RS: for<'de> serde::de::Deserialize<'de>,
    {
        let http_res = self.connector.request(request).await?;
        //let http_status = http_res.status();
        let http_body = hyper::body::aggregate(http_res).await?;
        let http_reader = http_body.reader();
        let decoded_body = serde_json::from_reader(http_reader)?;
        Ok(decoded_body)
    }

    pub fn open_session(&self, token: &SlackApiToken) -> SlackClientSession {
        SlackClientSession {
            client: &self,
            token: token.clone(),
        }
    }

    pub async fn get<RS, PT, TS>(&self, method_relative_uri: &str, params: PT) -> ClientResult<RS>
    where
        RS: for<'de> serde::de::Deserialize<'de>,
        PT: std::iter::IntoIterator<Item = (TS, Option<TS>)>,
        TS: std::string::ToString,
    {
        let full_uri = SlackClient::create_url_with_params(
            &SlackClient::create_method_uri_path(&method_relative_uri),
            params,
        );

        let body = self
            .send_webapi_request(Request::get(full_uri).body(Body::empty())?)
            .await?;

        Ok(body)
    }
}

impl<'a> SlackClientSession<'_> {
    fn setup_token_auth_header(
        &self,
        request_builder: hyper::http::request::Builder,
    ) -> hyper::http::request::Builder {
        let token_header_value = format!("Bearer {}", self.token.value);
        request_builder.header("Authorization", token_header_value)
    }

    pub async fn get<RS, PT, TS>(&self, method_relative_uri: &str, params: PT) -> ClientResult<RS>
    where
        RS: for<'de> serde::de::Deserialize<'de>,
        PT: std::iter::IntoIterator<Item = (TS, Option<TS>)>,
        TS: std::string::ToString,
    {
        let full_uri = SlackClient::create_url_with_params(
            &SlackClient::create_method_uri_path(&method_relative_uri),
            params,
        );

        let body = self
            .client
            .send_webapi_request(
                self.setup_token_auth_header(Request::get(full_uri))
                    .body(Body::empty())?,
            )
            .await?;

        Ok(body)
    }

    pub async fn post<RQ, RS, PT, TS>(
        &self,
        method_relative_uri: &str,
        request: RQ,
    ) -> ClientResult<RS>
    where
        RQ: serde::ser::Serialize,
        RS: for<'de> serde::de::Deserialize<'de>,
        PT: std::iter::IntoIterator<Item = (TS, Option<TS>)>,
        TS: std::string::ToString,
    {
        let full_uri =
            SlackClient::create_url(&SlackClient::create_method_uri_path(&method_relative_uri));

        let post_json = serde_json::to_string(&request)?;

        let response_body = self
            .client
            .send_webapi_request(
                self.setup_token_auth_header(Request::get(full_uri))
                    .header("content-type", "application/json")
                    .body(Body::from(post_json))?,
            )
            .await?;

        Ok(response_body)
    }
}
