//! `fetch_containers` / `fetch_host_info` 용 TTL 캐시 싱글턴.

use std::sync::OnceLock;
use std::time::Duration;

use crate::cache::TtlCache;
use super::types::{Container, HostInfo, ProxmoxServer};

const CONTAINER_CACHE_TTL: Duration = Duration::from_secs(10);
const HOST_INFO_CACHE_TTL: Duration = Duration::from_secs(5);

pub(crate) fn container_cache() -> &'static TtlCache<Vec<Container>> {
    static CACHE: OnceLock<TtlCache<Vec<Container>>> = OnceLock::new();
    CACHE.get_or_init(|| TtlCache::new(CONTAINER_CACHE_TTL))
}

pub(crate) fn host_info_cache() -> &'static TtlCache<HostInfo> {
    static CACHE: OnceLock<TtlCache<HostInfo>> = OnceLock::new();
    CACHE.get_or_init(|| TtlCache::new(HOST_INFO_CACHE_TTL))
}

pub(crate) fn cache_key(server: &ProxmoxServer) -> String {
    format!("{}@{}:{}", server.user, server.host, server.port)
}

pub(crate) fn invalidate_containers(server: &ProxmoxServer) {
    container_cache().invalidate(&cache_key(server));
}
