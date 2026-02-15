use rs_plugin_common_interfaces::{
    domain::{
        external_images::{ExternalImage, ImageType},
        serie::{Serie, SerieStatus},
    },
    lookup::RsLookupMetadataResult,
    lookup::RsLookupMetadataResultWithImages,
};
use serde_json::json;

use crate::anilist::AniListMedia;

fn best_title(media: &AniListMedia) -> String {
    media
        .title
        .as_ref()
        .and_then(|t| {
            t.english
                .clone()
                .or_else(|| t.romaji.clone())
                .or_else(|| t.native.clone())
        })
        .unwrap_or_default()
}

fn alt_names(media: &AniListMedia) -> Option<Vec<String>> {
    let primary = best_title(media);
    let mut alts: Vec<String> = Vec::new();

    if let Some(title) = &media.title {
        for name in [&title.english, &title.romaji, &title.native] {
            if let Some(n) = name {
                if *n != primary && !alts.contains(n) {
                    alts.push(n.clone());
                }
            }
        }
    }

    if let Some(synonyms) = &media.synonyms {
        for s in synonyms {
            if *s != primary && !alts.contains(s) {
                alts.push(s.clone());
            }
        }
    }

    if alts.is_empty() {
        None
    } else {
        Some(alts)
    }
}

fn map_status(status: &Option<String>) -> Option<SerieStatus> {
    status.as_ref().map(|s| match s.as_str() {
        "FINISHED" => SerieStatus::Ended,
        "RELEASING" => SerieStatus::Returning,
        "NOT_YET_RELEASED" => SerieStatus::Planned,
        "CANCELLED" => SerieStatus::Canceled,
        "HIATUS" => SerieStatus::Other("hiatus".to_string()),
        _ => SerieStatus::Unknown,
    })
}

fn build_trailer_url(media: &AniListMedia) -> Option<String> {
    media.trailer.as_ref().and_then(|t| {
        if t.site.as_deref() == Some("youtube") {
            t.id.as_ref()
                .map(|id| format!("https://www.youtube.com/watch?v={}", id))
        } else {
            None
        }
    })
}

fn build_images(media: &AniListMedia) -> Vec<ExternalImage> {
    let mut images = Vec::new();

    if let Some(cover) = &media.cover_image {
        let url = cover
            .extra_large
            .as_ref()
            .or(cover.large.as_ref())
            .or(cover.medium.as_ref());
        if let Some(url) = url {
            images.push(ExternalImage {
                kind: Some(ImageType::Poster),
                url: url.clone(),
                ..Default::default()
            });
        }
    }

    if let Some(banner) = &media.banner_image {
        images.push(ExternalImage {
            kind: Some(ImageType::Background),
            url: banner.clone(),
            ..Default::default()
        });
    }

    images
}

fn build_params(media: &AniListMedia) -> Option<serde_json::Value> {
    let mut params = serde_json::Map::new();

    if let Some(desc) = &media.description {
        params.insert("overview".to_string(), json!(desc));
    }
    if let Some(genres) = &media.genres {
        params.insert("genres".to_string(), json!(genres));
    }
    if let Some(country) = &media.country_of_origin {
        params.insert("country".to_string(), json!(country));
    }
    params.insert("anilist_id".to_string(), json!(media.id));
    if let Some(format) = &media.format {
        params.insert("format".to_string(), json!(format));
    }
    if let Some(episodes) = media.episodes {
        params.insert("episodes".to_string(), json!(episodes));
    }
    if let Some(score) = media.average_score {
        params.insert("averageScore".to_string(), json!(score));
    }
    if let Some(popularity) = media.popularity {
        params.insert("popularity".to_string(), json!(popularity));
    }
    if let Some(is_adult) = media.is_adult {
        params.insert("isAdult".to_string(), json!(is_adult));
    }
    if let Some(site_url) = &media.site_url {
        params.insert("siteUrl".to_string(), json!(site_url));
    }

    Some(serde_json::Value::Object(params))
}

pub fn anilist_media_to_result(media: AniListMedia) -> RsLookupMetadataResultWithImages {
    let images = build_images(&media);

    let serie = Serie {
        id: format!("anilist:{}", media.id),
        name: best_title(&media),
        kind: media.format.as_ref().map(|f| f.to_lowercase()),
        alt: alt_names(&media),
        status: map_status(&media.status),
        year: media.start_date.as_ref().and_then(|d| d.year),
        trailer: build_trailer_url(&media),
        anilist_manga_id: Some(media.id),
        myanimelist_manga_id: media.id_mal,
        params: build_params(&media),
        ..Default::default()
    };

    RsLookupMetadataResultWithImages {
        metadata: RsLookupMetadataResult::Serie(serie),
        images,
    }
}

pub fn anilist_media_to_images(media: &AniListMedia) -> Vec<ExternalImage> {
    build_images(media)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anilist::*;

    fn sample_media() -> AniListMedia {
        AniListMedia {
            id: 1535,
            title: Some(AniListTitle {
                romaji: Some("Shinseiki Evangelion".to_string()),
                english: Some("Neon Genesis Evangelion".to_string()),
                native: Some("\u{65b0}\u{4e16}\u{7d00}\u{30a8}\u{30f4}\u{30a1}\u{30f3}\u{30b2}\u{30ea}\u{30aa}\u{30f3}".to_string()),
            }),
            media_type: Some("ANIME".to_string()),
            format: Some("TV".to_string()),
            status: Some("FINISHED".to_string()),
            description: Some("A mecha anime.".to_string()),
            start_date: Some(AniListDate { year: Some(1995), month: Some(10), day: Some(4) }),
            cover_image: Some(AniListCoverImage {
                extra_large: Some("https://img.anilist.co/poster.jpg".to_string()),
                large: None,
                medium: None,
            }),
            banner_image: Some("https://img.anilist.co/banner.jpg".to_string()),
            genres: Some(vec!["Action".to_string(), "Mecha".to_string()]),
            synonyms: Some(vec!["NGE".to_string()]),
            average_score: Some(83),
            popularity: Some(200000),
            episodes: Some(26),
            trailer: Some(AniListTrailer {
                id: Some("dQw4w9WgXcQ".to_string()),
                site: Some("youtube".to_string()),
            }),
            country_of_origin: Some("JP".to_string()),
            id_mal: Some(30),
            site_url: Some("https://anilist.co/anime/1535".to_string()),
            is_adult: Some(false),
        }
    }

    #[test]
    fn test_best_title_prefers_english() {
        let media = sample_media();
        assert_eq!(best_title(&media), "Neon Genesis Evangelion");
    }

    #[test]
    fn test_best_title_falls_back_to_romaji() {
        let mut media = sample_media();
        media.title.as_mut().unwrap().english = None;
        assert_eq!(best_title(&media), "Shinseiki Evangelion");
    }

    #[test]
    fn test_alt_names_excludes_primary() {
        let media = sample_media();
        let alts = alt_names(&media).unwrap();
        assert!(!alts.contains(&"Neon Genesis Evangelion".to_string()));
        assert!(alts.contains(&"Shinseiki Evangelion".to_string()));
        assert!(alts.contains(&"NGE".to_string()));
    }

    #[test]
    fn test_status_mapping() {
        assert_eq!(
            map_status(&Some("FINISHED".to_string())),
            Some(SerieStatus::Ended)
        );
        assert_eq!(
            map_status(&Some("RELEASING".to_string())),
            Some(SerieStatus::Returning)
        );
        assert_eq!(
            map_status(&Some("NOT_YET_RELEASED".to_string())),
            Some(SerieStatus::Planned)
        );
        assert_eq!(
            map_status(&Some("CANCELLED".to_string())),
            Some(SerieStatus::Canceled)
        );
        assert_eq!(
            map_status(&Some("HIATUS".to_string())),
            Some(SerieStatus::Other("hiatus".to_string()))
        );
        assert_eq!(map_status(&None), None);
    }

    #[test]
    fn test_trailer_url_youtube() {
        let media = sample_media();
        assert_eq!(
            build_trailer_url(&media),
            Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string())
        );
    }

    #[test]
    fn test_trailer_url_non_youtube() {
        let mut media = sample_media();
        media.trailer.as_mut().unwrap().site = Some("dailymotion".to_string());
        assert_eq!(build_trailer_url(&media), None);
    }

    #[test]
    fn test_full_conversion() {
        let media = sample_media();
        let result = anilist_media_to_result(media);

        if let RsLookupMetadataResult::Serie(serie) = &result.metadata {
            assert_eq!(serie.id, "anilist:1535");
            assert_eq!(serie.name, "Neon Genesis Evangelion");
            assert_eq!(serie.kind, Some("tv".to_string()));
            assert_eq!(serie.year, Some(1995));
            assert_eq!(serie.anilist_manga_id, Some(1535));
            assert_eq!(serie.myanimelist_manga_id, Some(30));
            assert!(serie.trailer.is_some());
            assert!(serie.params.is_some());
        } else {
            panic!("Expected Serie metadata");
        }

        assert_eq!(result.images.len(), 2);
    }

    #[test]
    fn test_images_poster_fallback() {
        let mut media = sample_media();
        media.cover_image = Some(AniListCoverImage {
            extra_large: None,
            large: Some("https://img.anilist.co/large.jpg".to_string()),
            medium: None,
        });
        media.banner_image = None;

        let result = anilist_media_to_result(media);
        assert_eq!(result.images.len(), 1);
        assert_eq!(result.images[0].url, "https://img.anilist.co/large.jpg");
    }
}
