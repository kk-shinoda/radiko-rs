use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename = "urls")]
pub struct StreamUrlsXml {
    #[serde(rename = "url")]
    pub urls: Vec<StreamUrlXml>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StreamUrlXml {
    #[serde(rename = "@areafree")]
    pub areafree: String,
    #[serde(rename = "@timefree")]
    pub timefree: String,
    pub playlist_create_url: String,
}
