use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    api::{
        auth::RadikoAuthManager, program::RadikoProgram, station::RadikoStation,
        stream::RadikoStream,
    },
    models::{
        program::Programs, region::RegionStations, search::SearchCondition, station::Stations,
    },
};
use anyhow::Result;
use secrecy::{ExposeSecret, SecretString};

pub struct Radiko {
    inner: Arc<RwLock<RadikoRef>>,
}

struct RadikoRef {
    auth_manager: Arc<RadikoAuthManager>,
    stream: RadikoStream,
    station: RadikoStation,
    program: RadikoProgram,
    email: Option<SecretString>,
    password: Option<SecretString>,
}

impl Radiko {
    pub async fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Self::init_inner(None, None).await)),
        }
    }

    pub async fn new_area_free(email: &str, password: &str) -> Self {
        Self {
            inner: Arc::new(RwLock::new(
                Self::init_inner(
                    Some(SecretString::new(email.into())),
                    Some(SecretString::new(password.into())),
                )
                .await,
            )),
        }
    }

    async fn init_inner(email: Option<SecretString>, password: Option<SecretString>) -> RadikoRef {
        let is_area_free = email.is_some() && password.is_some();
        let shared_auth_manager = if is_area_free {
            Arc::new(
                RadikoAuthManager::new_area_free(
                    email.clone().unwrap().expose_secret(),
                    password.clone().unwrap().expose_secret(),
                )
                .await,
            )
        } else {
            Arc::new(RadikoAuthManager::new().await)
        };
        RadikoRef {
            auth_manager: Arc::clone(&shared_auth_manager),
            stream: RadikoStream::new(Arc::clone(&shared_auth_manager)),
            station: RadikoStation::new(),
            program: RadikoProgram::new(),
            email: email,
            password: password,
        }
    }

    pub async fn refresh_auth(&self) -> Result<()> {
        let email = self.inner.read().await.email.clone();
        let password = self.inner.read().await.password.clone();
        let mut inner = self.inner.write().await;
        *inner = Self::init_inner(email, password).await;
        drop(inner);
        Ok(())
    }

    pub async fn area_id(&self) -> String {
        self.inner.read().await.auth_manager.area_id().to_string()
    }

    pub async fn auth_token(&self) -> String {
        self.inner
            .read()
            .await
            .auth_manager
            .auth_token()
            .to_string()
    }

    pub async fn stream_url(&self, station_id: &str) -> String {
        self.inner
            .read()
            .await
            .stream
            .stream_url(station_id)
            .to_string()
    }

    /// タイムフリー再生用のストリームURLを取得
    /// ft: 開始日時 (YYYYMMDDHHmmss形式)
    /// to: 終了日時 (YYYYMMDDHHmmss形式)
    pub async fn timefree_stream_url(
        &self,
        station_id: &str,
        ft: &str,
        to: &str,
    ) -> Result<String> {
        self.inner
            .read()
            .await
            .stream
            .timefree_stream_url(station_id, ft, to)
            .await
    }

    pub async fn stations_all(&self) -> Result<Vec<RegionStations>> {
        self.inner.read().await.station.stations_all().await
    }

    pub async fn stations_from_area_id(&self, area_id: &str) -> Result<Stations> {
        self.inner
            .read()
            .await
            .station
            .stations_from_area_id(area_id)
            .await
    }

    pub async fn now_on_air_programs(&self, area_id: &str) -> Result<Programs> {
        self.inner
            .read()
            .await
            .program
            .now_on_air_programs(area_id)
            .await
    }

    pub async fn weekly_programs_from_station(&self, station_id: &str) -> Result<Programs> {
        self.inner
            .read()
            .await
            .program
            .weekly_programs_from_station(station_id)
            .await
    }

    pub async fn find_program(&self, search_condition: &SearchCondition) -> Result<Programs> {
        self.inner
            .read()
            .await
            .program
            .find_program(search_condition)
            .await
    }
}
