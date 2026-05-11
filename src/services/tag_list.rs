use std::time::{SystemTime, UNIX_EPOCH};

use dashmap::DashMap;

use crate::models::tag::{TagInput, TagRecord};

#[derive(Debug, Default)]
pub struct TagList {
    tags_by_id: DashMap<String, TagRecord>,
    epc_to_tid: DashMap<String, Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagListError {
    InvalidEpc,
    InvalidTid,
    InvalidAnt,
    InvalidRssi,
}

impl std::fmt::Display for TagListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidEpc => {
                write!(
                    f,
                    "epc must be non-empty, hexadecimal, and have a length multiple of 4"
                )
            }
            Self::InvalidTid => write!(f, "tid must be exactly 24 hexadecimal characters"),
            Self::InvalidAnt => write!(f, "ant must be a positive integer"),
            Self::InvalidRssi => write!(f, "rssi must be between -255 and -1"),
        }
    }
}

impl std::error::Error for TagListError {}

impl TagList {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn add(&self, input: TagInput) -> Result<TagRecord, TagListError> {
        let epc = normalize_hex(&input.epc);
        let tid = input.tid.as_deref().map(normalize_hex);

        validate_epc(&epc)?;
        validate_tid(tid.as_deref())?;
        validate_ant(input.ant)?;
        validate_rssi(input.rssi)?;

        let identifier = Self::build_identifier(&epc, tid.as_deref());
        let record = TagRecord {
            identifier: identifier.clone(),
            epc: epc.clone(),
            tid: tid.clone(),
            ant: input.ant,
            rssi: input.rssi,
            created_at_ms: current_timestamp_ms(),
        };

        if let Some((previous_epc, previous_tid)) = self
            .epc_to_tid
            .insert(epc.clone(), tid.clone())
            .map(|entry| (epc.clone(), entry))
        {
            let previous_identifier =
                Self::build_identifier(&previous_epc, previous_tid.as_deref());
            if previous_identifier != identifier {
                self.tags_by_id.remove(&previous_identifier);
            }
        }

        if let Some((old_epc, old_tid)) = self
            .tags_by_id
            .insert(identifier.clone(), record.clone())
            .map(|existing| (existing.epc, existing.tid))
        {
            if old_epc != epc || old_tid != tid {
                self.remove_epc_index_if_matches(&old_epc, old_tid.as_deref());
            }
        }

        Ok(record)
    }

    pub async fn get_all(&self) -> Vec<TagRecord> {
        let mut records = self
            .tags_by_id
            .iter()
            .map(|entry| entry.value().clone())
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            left.created_at_ms
                .cmp(&right.created_at_ms)
                .then_with(|| left.identifier.cmp(&right.identifier))
        });
        records
    }

    pub async fn get_by_id(&self, identifier: &str) -> Option<TagRecord> {
        let identifier = normalize_identifier(identifier);
        self.tags_by_id
            .get(&identifier)
            .map(|entry| entry.value().clone())
    }

    pub async fn get_by_epc(&self, epc: &str) -> Option<TagRecord> {
        let epc = normalize_hex(epc);
        let tid = self.epc_to_tid.get(&epc).map(|value| value.clone())?;
        let identifier = Self::build_identifier(&epc, tid.as_deref());
        self.get_by_id(&identifier).await
    }

    pub async fn get_by_tid(&self, tid: &str) -> Option<TagRecord> {
        let tid = normalize_hex(tid);
        self.get_by_id(&tid).await
    }

    pub async fn get_tid_by_epc(&self, epc: &str) -> Option<String> {
        let epc = normalize_hex(epc);
        self.epc_to_tid.get(&epc).and_then(|entry| entry.clone())
    }

    pub async fn remove(&self, identifier: &str) -> Option<TagRecord> {
        let identifier = normalize_identifier(identifier);
        let (_, removed) = self.tags_by_id.remove(&identifier)?;
        self.remove_epc_index_if_matches(&removed.epc, removed.tid.as_deref());
        Some(removed)
    }

    pub async fn remove_before_timestamp(&self, timestamp_ms: u128) -> Vec<TagRecord> {
        let identifiers = self
            .tags_by_id
            .iter()
            .filter_map(|entry| {
                (entry.value().created_at_ms < timestamp_ms).then(|| entry.key().clone())
            })
            .collect::<Vec<_>>();

        let mut removed = Vec::with_capacity(identifiers.len());
        for identifier in identifiers {
            if let Some(record) = self.remove(&identifier).await {
                removed.push(record);
            }
        }
        removed.sort_by(|left, right| left.created_at_ms.cmp(&right.created_at_ms));
        removed
    }

    pub fn build_identifier(epc: &str, tid: Option<&str>) -> String {
        tid.map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("_{}", epc))
    }

    fn remove_epc_index_if_matches(&self, epc: &str, tid: Option<&str>) {
        let expected_tid = tid.map(str::to_owned);
        if self
            .epc_to_tid
            .get(epc)
            .is_some_and(|entry| *entry == expected_tid)
        {
            self.epc_to_tid.remove(epc);
        }
    }
}

fn current_timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is set before unix epoch")
        .as_millis()
}

fn normalize_hex(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

fn normalize_identifier(value: &str) -> String {
    let trimmed = value.trim();
    if let Some(epc) = trimmed.strip_prefix('_') {
        format!("_{}", normalize_hex(epc))
    } else {
        normalize_hex(trimmed)
    }
}

fn validate_epc(epc: &str) -> Result<(), TagListError> {
    if epc.is_empty() || epc.len() % 4 != 0 || !epc.chars().all(|char| char.is_ascii_hexdigit()) {
        return Err(TagListError::InvalidEpc);
    }
    Ok(())
}

fn validate_tid(tid: Option<&str>) -> Result<(), TagListError> {
    match tid {
        Some(value) if value.len() == 24 && value.chars().all(|char| char.is_ascii_hexdigit()) => {
            Ok(())
        }
        Some(_) => Err(TagListError::InvalidTid),
        None => Ok(()),
    }
}

fn validate_ant(ant: i32) -> Result<(), TagListError> {
    if ant <= 0 {
        return Err(TagListError::InvalidAnt);
    }
    Ok(())
}

fn validate_rssi(rssi: i32) -> Result<(), TagListError> {
    if !(-255..=-1).contains(&rssi) {
        return Err(TagListError::InvalidRssi);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{TagInput, TagList, TagListError};

    #[tokio::test]
    async fn add_without_tid_uses_epc_identifier() {
        let tag_list = TagList::new();
        let created = tag_list
            .add(TagInput {
                epc: "abcd1234".into(),
                tid: None,
                ant: 1,
                rssi: -42,
            })
            .await
            .expect("tag should be created");

        assert_eq!(created.identifier, "_ABCD1234");
        assert_eq!(tag_list.get_by_epc("ABCD1234").await, Some(created.clone()));
        assert_eq!(tag_list.get_tid_by_epc("abcd1234").await, None);
    }

    #[tokio::test]
    async fn add_with_tid_updates_secondary_index() {
        let tag_list = TagList::new();
        let created = tag_list
            .add(TagInput {
                epc: "abcd1234".into(),
                tid: Some("00112233445566778899aabb".into()),
                ant: 2,
                rssi: -30,
            })
            .await
            .expect("tag should be created");

        assert_eq!(created.identifier, "00112233445566778899AABB");
        assert_eq!(
            tag_list.get_by_tid("00112233445566778899aabb").await,
            Some(created.clone())
        );
        assert_eq!(tag_list.get_by_epc("abcd1234").await, Some(created.clone()));
        assert_eq!(
            tag_list.get_tid_by_epc("ABCD1234").await,
            Some("00112233445566778899AABB".into())
        );
    }

    #[tokio::test]
    async fn remove_before_timestamp_only_removes_older_tags() {
        let tag_list = TagList::new();
        let first = tag_list
            .add(TagInput {
                epc: "AAAA".into(),
                tid: None,
                ant: 1,
                rssi: -10,
            })
            .await
            .expect("first tag should be created");

        tokio::time::sleep(Duration::from_millis(2)).await;
        let second = tag_list
            .add(TagInput {
                epc: "BBBB".into(),
                tid: Some("ABCDEF0123456789ABCDEF01".into()),
                ant: 1,
                rssi: -20,
            })
            .await
            .expect("second tag should be created");

        let removed = tag_list.remove_before_timestamp(second.created_at_ms).await;

        assert_eq!(removed, vec![first]);
        assert_eq!(tag_list.get_by_epc("AAAA").await, None);
        assert_eq!(tag_list.get_by_id(&second.identifier).await, Some(second));
    }

    #[tokio::test]
    async fn rejects_invalid_payloads() {
        let tag_list = TagList::new();

        let err = tag_list
            .add(TagInput {
                epc: "XYZ".into(),
                tid: None,
                ant: 1,
                rssi: -10,
            })
            .await
            .expect_err("invalid epc must fail");
        assert_eq!(err, TagListError::InvalidEpc);

        let err = tag_list
            .add(TagInput {
                epc: "ABCD".into(),
                tid: Some("1234".into()),
                ant: 1,
                rssi: -10,
            })
            .await
            .expect_err("invalid tid must fail");
        assert_eq!(err, TagListError::InvalidTid);

        let err = tag_list
            .add(TagInput {
                epc: "ABCD".into(),
                tid: None,
                ant: 0,
                rssi: -10,
            })
            .await
            .expect_err("invalid ant must fail");
        assert_eq!(err, TagListError::InvalidAnt);

        let err = tag_list
            .add(TagInput {
                epc: "ABCD".into(),
                tid: None,
                ant: 1,
                rssi: 0,
            })
            .await
            .expect_err("invalid rssi must fail");
        assert_eq!(err, TagListError::InvalidRssi);
    }
}
