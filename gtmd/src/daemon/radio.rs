use super::*;

/// Radio Browser directory integration.
pub(crate) struct Radio;

fn radio_cache_path(inner: &DaemonInner, name: &str) -> std::path::PathBuf {
    inner.config.cache_dir.join(format!("radio_{name}.json"))
}

fn read_cache<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Option<T> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

fn write_cache<T: serde::Serialize>(path: &std::path::Path, val: &T) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(data) = serde_json::to_vec(val) {
        let _ = std::fs::write(path, data);
    }
}

impl Radio {
    pub async fn search(
        inner: &DaemonInner,
        query: &str,
        limit: u16,
    ) -> Result<DaemonRes, CoreError> {
        let radio = inner.radio.lock().await;
        let stations = radio
            .search(query, limit)
            .await
            .map_err(CoreError::Daemon)?;
        Ok(DaemonRes::RadioStationsRes { stations })
    }

    pub async fn top(inner: &DaemonInner, limit: u16) -> Result<DaemonRes, CoreError> {
        let cache = radio_cache_path(inner, &format!("top_{limit}"));
        let radio = inner.radio.lock().await;
        match radio.top(limit).await {
            Ok(stations) => {
                write_cache(&cache, &stations);
                Ok(DaemonRes::RadioStationsRes { stations })
            }
            Err(e) => {
                if let Some(stations) = read_cache(&cache) {
                    Ok(DaemonRes::RadioStationsRes { stations })
                } else {
                    Err(CoreError::Daemon(e))
                }
            }
        }
    }

    pub async fn cliamp(inner: &DaemonInner, refresh: bool) -> Result<DaemonRes, CoreError> {
        let cache = radio_cache_path(inner, "cliamp");
        let radio = inner.radio.lock().await;
        match radio.cliamp(refresh).await {
            Ok(stations) => {
                write_cache(&cache, &stations);
                Ok(DaemonRes::RadioStationsRes { stations })
            }
            Err(e) => {
                if let Some(stations) = read_cache(&cache) {
                    Ok(DaemonRes::RadioStationsRes { stations })
                } else {
                    Err(CoreError::Daemon(e))
                }
            }
        }
    }

    pub async fn tags(inner: &DaemonInner, limit: u16) -> Result<DaemonRes, CoreError> {
        let cache = radio_cache_path(inner, &format!("tags_{limit}"));
        let radio = inner.radio.lock().await;
        match radio.tags(limit).await {
            Ok(tags) => {
                write_cache(&cache, &tags);
                Ok(DaemonRes::RadioTagsRes { tags })
            }
            Err(e) => {
                if let Some(tags) = read_cache(&cache) {
                    Ok(DaemonRes::RadioTagsRes { tags })
                } else {
                    Err(CoreError::Daemon(e))
                }
            }
        }
    }

    pub async fn by_tag(
        inner: &DaemonInner,
        tag: &str,
        limit: u16,
    ) -> Result<DaemonRes, CoreError> {
        let radio = inner.radio.lock().await;
        let stations = radio
            .stations_by_tag(tag, limit)
            .await
            .map_err(CoreError::Daemon)?;
        Ok(DaemonRes::RadioStationsRes { stations })
    }

    pub async fn countries(inner: &DaemonInner, limit: u16) -> Result<DaemonRes, CoreError> {
        let cache = radio_cache_path(inner, &format!("countries_{limit}"));
        let radio = inner.radio.lock().await;
        match radio.countries(limit).await {
            Ok(countries) => {
                write_cache(&cache, &countries);
                Ok(DaemonRes::RadioCountriesRes { countries })
            }
            Err(e) => {
                if let Some(countries) = read_cache(&cache) {
                    Ok(DaemonRes::RadioCountriesRes { countries })
                } else {
                    Err(CoreError::Daemon(e))
                }
            }
        }
    }

    pub async fn by_country(
        inner: &DaemonInner,
        country: &str,
        limit: u16,
    ) -> Result<DaemonRes, CoreError> {
        let radio = inner.radio.lock().await;
        let stations = radio
            .stations_by_country(country, limit)
            .await
            .map_err(CoreError::Daemon)?;
        Ok(DaemonRes::RadioStationsRes { stations })
    }

    pub async fn play(
        inner: &DaemonInner,
        station_id: &str,
        station_name: &str,
    ) -> Result<DaemonRes, CoreError> {
        // Don't push radio into the queue - live streams don't belong there.
        // The name rides in the path because `parse_remote_path` reads it back
        // out to label the track: a station published as `radio://<id>` alone
        // has no name to resolve, and the now-playing line came up empty
        // whenever the stream published no ICY title to stand in for it.
        let path = match station_name.trim() {
            "" => format!("radio://{station_id}"),
            name => format!("radio://{station_id}/{name}"),
        };
        Cmd::play(inner, &path, 0.0, false).await
    }
}
