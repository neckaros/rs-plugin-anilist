use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphQLRequest {
    pub query: String,
    pub variables: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct AniListSearchResponse {
    pub data: Option<AniListSearchData>,
}

#[derive(Debug, Deserialize)]
pub struct AniListSearchData {
    pub sfw: Option<AniListPage>,
    pub nsfw: Option<AniListPage>,
}

#[derive(Debug, Deserialize)]
pub struct AniListIdResponse {
    pub data: Option<AniListIdData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AniListIdData {
    pub media: Option<AniListMedia>,
}

#[derive(Debug, Deserialize)]
pub struct AniListPage {
    pub media: Option<Vec<AniListMedia>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AniListMedia {
    pub id: u64,
    pub title: Option<AniListTitle>,
    #[serde(rename = "type")]
    pub media_type: Option<String>,
    pub format: Option<String>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub start_date: Option<AniListDate>,
    pub cover_image: Option<AniListCoverImage>,
    pub banner_image: Option<String>,
    pub genres: Option<Vec<String>>,
    pub synonyms: Option<Vec<String>>,
    pub average_score: Option<u32>,
    pub popularity: Option<u64>,
    pub episodes: Option<u32>,
    pub trailer: Option<AniListTrailer>,
    pub country_of_origin: Option<String>,
    pub id_mal: Option<u64>,
    pub site_url: Option<String>,
    pub is_adult: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AniListTitle {
    pub romaji: Option<String>,
    pub english: Option<String>,
    pub native: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AniListDate {
    pub year: Option<u16>,
    pub month: Option<u8>,
    pub day: Option<u8>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AniListCoverImage {
    pub extra_large: Option<String>,
    pub large: Option<String>,
    pub medium: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AniListTrailer {
    pub id: Option<String>,
    pub site: Option<String>,
}

const MEDIA_FIELDS: &str = "
      id
      title {
        romaji
        english
        native
      }
      type
      format
      status
      description
      startDate {
        year
        month
        day
      }
      coverImage {
        extraLarge
        large
        medium
      }
      bannerImage
      genres
      synonyms
      averageScore
      popularity
      episodes
      trailer {
        id
        site
      }
      countryOfOrigin
      idMal
      siteUrl
      isAdult
";

pub fn build_id_query() -> String {
    format!(
        r#"query ($id: Int) {{
  Media(id: $id) {{
    {fields}
  }}
}}"#,
        fields = MEDIA_FIELDS
    )
}

pub fn build_search_query() -> String {
    format!(
        r#"query ($search: String, $type: MediaType) {{
  sfw: Page(page: 1, perPage: 25) {{
    media(search: $search, type: $type, isAdult: false) {{
      {fields}
    }}
  }}
  nsfw: Page(page: 1, perPage: 25) {{
    media(search: $search, type: $type, isAdult: true) {{
      {fields}
    }}
  }}
}}"#,
        fields = MEDIA_FIELDS
    )
}
