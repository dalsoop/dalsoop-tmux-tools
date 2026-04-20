//! 간단한 TTL 인메모리 캐시.
//!
//! Proxmox 탭이 반복 호출하는 fetch 계열 함수에서 공유. 키는 \`String\` 고정,
//! 값 타입은 제네릭. 만료된 엔트리는 get 시점에 lazy 로 걸러낸다.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct TtlCache<V> {
    inner: Mutex<HashMap<String, (Instant, V)>>,
    ttl: Duration,
}

impl<V: Clone> TtlCache<V> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    /// TTL 내 값이 있으면 복제해서 반환, 없거나 만료됐으면 `None`.
    pub fn get(&self, key: &str) -> Option<V> {
        let map = self.inner.lock().ok()?;
        let (at, v) = map.get(key)?;
        if at.elapsed() < self.ttl {
            Some(v.clone())
        } else {
            None
        }
    }

    /// 새 값을 넣거나 덮어쓴다.
    pub fn put(&self, key: String, value: V) {
        if let Ok(mut map) = self.inner.lock() {
            map.insert(key, (Instant::now(), value));
        }
    }

    /// 특정 키 무효화.
    pub fn invalidate(&self, key: &str) {
        if let Ok(mut map) = self.inner.lock() {
            map.remove(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn get_after_put_hits() {
        let c: TtlCache<i32> = TtlCache::new(Duration::from_secs(60));
        c.put("x".into(), 42);
        assert_eq!(c.get("x"), Some(42));
    }

    #[test]
    fn get_empty_is_none() {
        let c: TtlCache<String> = TtlCache::new(Duration::from_secs(60));
        assert_eq!(c.get("nope"), None);
    }

    #[test]
    fn expired_entry_returns_none() {
        let c: TtlCache<i32> = TtlCache::new(Duration::from_millis(20));
        c.put("x".into(), 1);
        sleep(Duration::from_millis(50));
        assert_eq!(c.get("x"), None);
    }

    #[test]
    fn invalidate_removes() {
        let c: TtlCache<i32> = TtlCache::new(Duration::from_secs(60));
        c.put("x".into(), 1);
        c.invalidate("x");
        assert_eq!(c.get("x"), None);
    }

    #[test]
    fn put_overwrites() {
        let c: TtlCache<i32> = TtlCache::new(Duration::from_secs(60));
        c.put("x".into(), 1);
        c.put("x".into(), 2);
        assert_eq!(c.get("x"), Some(2));
    }
}
