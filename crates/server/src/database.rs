//! Bounded database pools, including a persistent connection for volatile SQLite.
use sea_orm::ConnectOptions;
use std::time::Duration;

pub fn options(url: String) -> ConnectOptions {
    let memory_database = url == "sqlite::memory:" || url.contains("mode=memory");
    let mut options = ConnectOptions::new(url);
    options
        .min_connections(if memory_database { 1 } else { 0 })
        .max_connections(if memory_database { 1 } else { 4 })
        .connect_timeout(Duration::from_secs(5))
        .acquire_timeout(Duration::from_secs(5))
        .sqlx_logging(false);
    if memory_database {
        // Retiring the last SQLite connection would erase the in-memory schema and data.
        options.idle_timeout(None).max_lifetime(None);
    }
    options
}

#[cfg(test)]
mod tests {
    use super::options;

    #[test]
    fn test_memory_pool_should_keep_its_only_connection_without_retirement() {
        for url in [
            "sqlite::memory:",
            "sqlite://volatile?mode=memory&cache=shared",
        ] {
            let options = options(url.to_owned());
            assert_eq!(options.get_min_connections(), Some(1));
            assert_eq!(options.get_max_connections(), Some(1));
            assert_eq!(options.get_idle_timeout(), Some(None));
            assert_eq!(options.get_max_lifetime(), Some(None));
        }
    }
}
