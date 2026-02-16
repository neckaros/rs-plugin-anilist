use extism_pdk::{http, log, plugin_fn, FnResult, HttpRequest, Json, LogLevel, WithReturnCode};
use std::collections::HashSet;

use rs_plugin_common_interfaces::{
    domain::external_images::ExternalImage,
    lookup::{RsLookupMetadataResultWithImages, RsLookupQuery, RsLookupWrapper},
    PluginCredential, PluginInformation, PluginType,
};
use serde_json::json;

mod anilist;
mod convert;

use anilist::{
    build_id_images_query, build_id_query, build_search_images_query, build_search_query,
    AniListIdResponse, AniListMedia, AniListSearchResponse, GraphQLRequest,
};
use convert::{anilist_media_to_images, anilist_media_to_result};

#[plugin_fn]
pub fn infos() -> FnResult<Json<PluginInformation>> {
    Ok(Json(PluginInformation {
        name: "anilist_metadata".into(),
        capabilities: vec![PluginType::LookupMetadata],
        version: 2,
        interface_version: 1,
        repo: Some("https://github.com/neckaros/rs-plugin-anilist".into()),
        publisher: "neckaros".into(),
        description: "Look up anime and manga metadata from AniList".into(),
        credential_kind: None,
        settings: vec![],
        ..Default::default()
    }))
}

fn extract_anilist_id(query: &RsLookupQuery) -> Option<u64> {
    match query {
        RsLookupQuery::Serie(s) => s.ids.as_ref().and_then(|ids| ids.anilist_manga_id),
        RsLookupQuery::Movie(m) => m.ids.as_ref().and_then(|ids| ids.anilist_manga_id),
        _ => None,
    }
}

fn build_http_request(credential: &Option<PluginCredential>) -> HttpRequest {
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

    if let Some(credential) = credential {
        if let Some(token) = &credential.password {
            request
                .headers
                .insert("Authorization".to_string(), format!("Bearer {}", token));
        }
    }

    request
}

fn fetch_by_id(id: u64, credential: &Option<PluginCredential>) -> FnResult<Vec<AniListMedia>> {
    execute_id_query(build_id_query(), id, credential)
}

fn fetch_by_search(
    search: &str,
    media_type: &str,
    credential: &Option<PluginCredential>,
) -> FnResult<Vec<AniListMedia>> {
    execute_search_query(build_search_query(), search, media_type, credential)
}

fn fetch_images_by_id(
    id: u64,
    credential: &Option<PluginCredential>,
) -> FnResult<Vec<AniListMedia>> {
    execute_id_query(build_id_images_query(), id, credential)
}

fn fetch_images_by_search(
    search: &str,
    media_type: &str,
    credential: &Option<PluginCredential>,
) -> FnResult<Vec<AniListMedia>> {
    execute_search_query(build_search_images_query(), search, media_type, credential)
}

fn execute_id_query(
    query: String,
    id: u64,
    credential: &Option<PluginCredential>,
) -> FnResult<Vec<AniListMedia>> {
    let body = GraphQLRequest {
        query,
        variables: json!({ "id": id }),
    };

    let request = build_http_request(credential);
    let body_json = serde_json::to_vec(&body).map_err(|e| {
        WithReturnCode::new(
            extism_pdk::Error::msg(format!("Serialize error: {}", e)),
            500,
        )
    })?;

    let res = http::request::<Vec<u8>>(&request, Some(body_json));

    match res {
        Ok(res) if res.status_code() >= 200 && res.status_code() < 300 => {
            match res.json::<AniListIdResponse>() {
                Ok(response) => {
                    let media = response.data.and_then(|d| d.media).into_iter().collect();
                    Ok(media)
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

fn execute_search_query(
    query: String,
    search: &str,
    media_type: &str,
    credential: &Option<PluginCredential>,
) -> FnResult<Vec<AniListMedia>> {
    let body = GraphQLRequest {
        query,
        variables: json!({
            "search": search,
            "type": media_type
        }),
    };

    let request = build_http_request(credential);
    let body_json = serde_json::to_vec(&body).map_err(|e| {
        WithReturnCode::new(
            extism_pdk::Error::msg(format!("Serialize error: {}", e)),
            500,
        )
    })?;

    let res = http::request::<Vec<u8>>(&request, Some(body_json));

    match res {
        Ok(res) if res.status_code() >= 200 && res.status_code() < 300 => {
            match res.json::<AniListSearchResponse>() {
                Ok(response) => {
                    let mut all_media = Vec::new();
                    if let Some(data) = response.data {
                        if let Some(sfw) = data.sfw {
                            all_media.extend(sfw.media.unwrap_or_default());
                        }
                        if let Some(nsfw) = data.nsfw {
                            all_media.extend(nsfw.media.unwrap_or_default());
                        }
                    }
                    Ok(all_media)
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

#[plugin_fn]
pub fn lookup_metadata(
    Json(lookup): Json<RsLookupWrapper>,
) -> FnResult<Json<Vec<RsLookupMetadataResultWithImages>>> {
    let all_media = lookup_media(&lookup)?;

    let results: Vec<RsLookupMetadataResultWithImages> =
        all_media.into_iter().map(anilist_media_to_result).collect();

    Ok(Json(results))
}

fn lookup_media(lookup: &RsLookupWrapper) -> FnResult<Vec<AniListMedia>> {
    lookup_media_with_fetchers(lookup, fetch_by_id, fetch_by_search)
}

fn lookup_media_images(lookup: &RsLookupWrapper) -> FnResult<Vec<AniListMedia>> {
    lookup_media_with_fetchers(lookup, fetch_images_by_id, fetch_images_by_search)
}

fn lookup_media_with_fetchers(
    lookup: &RsLookupWrapper,
    fetch_by_id_fn: fn(u64, &Option<PluginCredential>) -> FnResult<Vec<AniListMedia>>,
    fetch_by_search_fn: fn(&str, &str, &Option<PluginCredential>) -> FnResult<Vec<AniListMedia>>,
) -> FnResult<Vec<AniListMedia>> {
    let media_types = match media_types_for_query(&lookup.query) {
        Some(types) => types,
        None => return Ok(vec![]),
    };

    let all_media = if let Some(anilist_id) = extract_anilist_id(&lookup.query) {
        fetch_by_id_fn(anilist_id, &lookup.credential)?
    } else {
        let search = match &lookup.query {
            RsLookupQuery::Serie(s) => s.name.as_deref(),
            RsLookupQuery::Movie(m) => m.name.as_deref(),
            _ => unreachable!(),
        };
        match search {
            Some(s) if !s.trim().is_empty() => {
                fetch_across_media_types(s, media_types, &lookup.credential, fetch_by_search_fn)?
            }
            _ => {
                return Err(WithReturnCode::new(
                    extism_pdk::Error::msg("Not supported"),
                    404,
                ))
            }
        }
    };

    Ok(all_media
        .into_iter()
        .filter(|media| media_matches_query(&lookup.query, media))
        .collect())
}

fn media_types_for_query(query: &RsLookupQuery) -> Option<&'static [&'static str]> {
    match query {
        RsLookupQuery::Serie(_) => Some(&["ANIME", "MANGA"]),
        RsLookupQuery::Movie(_) => Some(&["ANIME"]),
        _ => None,
    }
}

fn fetch_across_media_types(
    search: &str,
    media_types: &[&str],
    credential: &Option<PluginCredential>,
    fetch_by_search_fn: fn(&str, &str, &Option<PluginCredential>) -> FnResult<Vec<AniListMedia>>,
) -> FnResult<Vec<AniListMedia>> {
    let mut all_media = Vec::new();
    let mut seen_ids = HashSet::new();

    for media_type in media_types {
        let media = fetch_by_search_fn(search, media_type, credential)?;
        for item in media {
            if seen_ids.insert(item.id) {
                all_media.push(item);
            }
        }
    }

    Ok(all_media)
}

fn media_matches_query(query: &RsLookupQuery, media: &AniListMedia) -> bool {
    let format = media.format.as_deref().unwrap_or_default();
    match query {
        RsLookupQuery::Serie(_) => is_serie_format(format),
        RsLookupQuery::Movie(_) => is_movie_format(format),
        _ => false,
    }
}

fn is_serie_format(format: &str) -> bool {
    matches!(
        format,
        "TV" | "TV_SHORT" | "ONA" | "OVA" | "SPECIAL" | "MANGA" | "NOVEL"
    )
}

fn is_movie_format(format: &str) -> bool {
    matches!(format, "MOVIE" | "OVA")
}

#[plugin_fn]
pub fn lookup_metadata_images(
    Json(lookup): Json<RsLookupWrapper>,
) -> FnResult<Json<Vec<ExternalImage>>> {
    let all_media = lookup_media_images(&lookup)?;

    let images: Vec<ExternalImage> = all_media
        .into_iter()
        .flat_map(|media| anilist_media_to_images(&media))
        .collect();

    Ok(Json(images))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anilist::AniListMedia;

    fn media_with_format(format: Option<&str>) -> AniListMedia {
        AniListMedia {
            id: 1,
            title: None,
            media_type: None,
            format: format.map(str::to_string),
            status: None,
            description: None,
            start_date: None,
            cover_image: None,
            banner_image: None,
            genres: None,
            synonyms: None,
            average_score: None,
            popularity: None,
            episodes: None,
            trailer: None,
            country_of_origin: None,
            id_mal: None,
            site_url: None,
            is_adult: None,
        }
    }

    #[test]
    fn serie_query_accepts_anime_and_manga_formats() {
        let query = RsLookupQuery::Serie(Default::default());
        assert!(media_matches_query(&query, &media_with_format(Some("TV"))));
        assert!(media_matches_query(
            &query,
            &media_with_format(Some("MANGA"))
        ));
        assert!(!media_matches_query(
            &query,
            &media_with_format(Some("MOVIE"))
        ));
    }

    #[test]
    fn movie_query_accepts_movie_and_ova_formats() {
        let query = RsLookupQuery::Movie(Default::default());
        assert!(media_matches_query(
            &query,
            &media_with_format(Some("MOVIE"))
        ));
        assert!(media_matches_query(&query, &media_with_format(Some("OVA"))));
        assert!(!media_matches_query(&query, &media_with_format(Some("TV"))));
    }

    #[test]
    fn serie_query_uses_anime_and_manga_media_types() {
        let types = media_types_for_query(&RsLookupQuery::Serie(Default::default())).unwrap();
        assert_eq!(types, &["ANIME", "MANGA"]);
    }
}
