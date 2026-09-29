//! Lookup API calls.

use proto::lookup::v1::group_snapshot::Member;

use crate::Error;
use crate::response::{
    LookupGroup, LookupGroupResult, LookupMembers, LookupPerson, LookupResponse,
};

const LOOKUP_API: &str = "https://api.apps.cam.ac.uk/lookup/v1";
const TOKEN_URL: &str = "https://api.apps.cam.ac.uk/oauth2/v1/token";
const LOOKUP_SCOPE: &str = "https://api.apps.cam.ac.uk/lookup";

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
}

/// Exchange API Gateway app credentials for an access token.
pub(crate) async fn fetch_access_token(
    http_client: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
) -> Result<String, Error> {
    let response: TokenResponse = http_client
        .post(TOKEN_URL)
        .basic_auth(client_id, Some(client_secret))
        .header("Accept", "application/json")
        .form(&[
            ("grant_type", "client_credentials"),
            ("scope", LOOKUP_SCOPE),
        ])
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(Error::Token)?
        .json()
        .await
        .map_err(Error::Token)?;

    Ok(response.access_token)
}

/// Send a GET request and deserialize to type T.
async fn get<T: serde::de::DeserializeOwned>(
    http_client: &reqwest::Client,
    token: &str,
    path: &str,
) -> Result<T, reqwest::Error> {
    http_client
        .get(format!("{LOOKUP_API}/{path}"))
        .bearer_auth(token)
        .header("Accept", "application/json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}

/// Fetch the description of the group from Lookup.
pub(crate) async fn fetch_group(
    http_client: &reqwest::Client,
    token: &str,
    group_id: &str,
) -> Result<LookupGroup, Error> {
    let path = format!("group/{group_id}");
    let response: LookupResponse<LookupGroupResult> = get(http_client, token, &path)
        .await
        .map_err(|source| Error::Lookup {
            group_id: group_id.to_owned(),
            source,
        })?;

    Ok(response.result.group)
}

/// Fetch the members of the group from Lookup.
pub(crate) async fn fetch_group_members(
    http_client: &reqwest::Client,
    token: &str,
    group_id: &str,
) -> Result<Vec<Member>, Error> {
    let path = format!("group/{group_id}/members");
    let members: Vec<Member> = get::<LookupResponse<LookupMembers>>(http_client, token, &path)
        .await
        .map_err(|source| Error::Lookup {
            group_id: group_id.to_owned(),
            source,
        })?
        .result
        .people
        .into_iter()
        .filter_map(LookupPerson::into_proto_member)
        .collect();

    if members.is_empty() {
        return Err(Error::EmptyGroup {
            group_id: group_id.to_owned(),
        });
    }

    Ok(members)
}
