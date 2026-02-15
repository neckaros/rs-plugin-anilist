use extism_pdk::{http, log, plugin_fn, FnResult, HttpRequest, Json, LogLevel, WithReturnCode};

use rs_plugin_common_interfaces::{
    lookup::{RsLookupMetadataResultWithImages, RsLookupQuery, RsLookupWrapper},
    PluginInformation, PluginType,
};
use serde_json::json;

mod anilist;
mod convert;

use anilist::{AniListGraphQLResponse, GraphQLRequest, build_search_query};
use convert::anilist_media_to_result;

#[plugin_fn]
pub fn infos() -> FnResult<Json<PluginInformation>> {
    Ok(Json(PluginInformation {
        name: "anilist_metadata".into(),
        capabilities: vec![PluginType::LookupMetadata],
        version: 1,
        interface_version: 1,
        repo: Some("https://github.com/neckaros/rs-plugin-anilist".into()),
        publisher: "neckaros".into(),
        description: "Look up anime and manga metadata from AniList".into(),
        credential_kind: None,
        settings: vec![],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn lookup_metadata(
    Json(lookup): Json<RsLookupWrapper>,
) -> FnResult<Json<Vec<RsLookupMetadataResultWithImages>>> {
    let (search, media_type) = match &lookup.query {
        RsLookupQuery::Serie(s) => (s.name.clone(), "ANIME"),
        RsLookupQuery::Movie(m) => (m.name.clone(), "ANIME"),
        _ => return Ok(Json(vec![])),
    };

    let body = GraphQLRequest {
        query: build_search_query(),
        variables: json!({
            "search": search,
            "type": media_type
        }),
    };

    let mut request = HttpRequest {
        url: "https://graphql.anilist.co".to_string(),
        headers: Default::default(),
        method: Some("POST".into()),
    };
    request
        .headers
        .insert("Content-Type".to_string(), "application/json".to_string());
    request
        .headers
        .insert("Accept".to_string(), "application/json".to_string());

    if let Some(credential) = &lookup.credential {
        if let Some(token) = &credential.password {
            request.headers.insert(
                "Authorization".to_string(),
                format!("Bearer {}", token),
            );
        }
    }

    let body_json = serde_json::to_vec(&body)
        .map_err(|e| WithReturnCode::new(extism_pdk::Error::msg(format!("Serialize error: {}", e)), 500))?;

    let res = http::request::<Vec<u8>>(&request, Some(body_json));

    match res {
        Ok(res) if res.status_code() >= 200 && res.status_code() < 300 => {
            match res.json::<AniListGraphQLResponse>() {
                Ok(graphql_response) => {
                    let mut all_media = Vec::new();
                    if let Some(data) = graphql_response.data {
                        if let Some(sfw) = data.sfw {
                            all_media.extend(sfw.media.unwrap_or_default());
                        }
                        if let Some(nsfw) = data.nsfw {
                            all_media.extend(nsfw.media.unwrap_or_default());
                        }
                    }
                    let results: Vec<RsLookupMetadataResultWithImages> = all_media
                        .into_iter()
                        .map(anilist_media_to_result)
                        .collect();
                    Ok(Json(results))
                }
                Err(e) => {
                    log!(LogLevel::Error, "AniList JSON parse error: {}", e);
                    Err(WithReturnCode::new(e, 500))
                }
            }
        }
        Ok(res) => {
            log!(
                LogLevel::Error,
                "AniList HTTP error {}: {}",
                res.status_code(),
                String::from_utf8_lossy(&res.body())
            );
            Err(WithReturnCode::new(
                extism_pdk::Error::msg(format!("HTTP error: {}", res.status_code())),
                res.status_code() as i32,
            ))
        }
        Err(e) => {
            log!(LogLevel::Error, "AniList request failed: {}", e);
            Err(WithReturnCode(e, 500))
        }
    }
}
