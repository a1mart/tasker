// src/types/timestamp.rs
use prost::{Message, DecodeError};
use prost_types::Timestamp;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Utc};

mod timestamp_serde {
    use super::*;
    
    pub fn serialize<S>(ts: &Timestamp, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        let dt = UNIX_EPOCH
            .checked_add(Duration::new(ts.seconds as u64, ts.nanos as u32))
            .ok_or_else(|| serde::ser::Error::custom("invalid timestamp"))?;
        let datetime: DateTime<Utc> = dt.into();
        datetime.to_rfc3339().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Timestamp, D::Error>
    where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        let dt = DateTime::parse_from_rfc3339(&s)
            .map_err(serde::de::Error::custom)?
            .with_timezone(&Utc);
        Ok(Timestamp {
            seconds: dt.timestamp(),
            nanos: dt.timestamp_subsec_nanos() as i32,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SerdeTimestamp(
    #[serde(with = "timestamp_serde")]
    pub Timestamp,
);

// Implement prost::Message for SerdeTimestamp
impl Message for SerdeTimestamp {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: prost::bytes::BufMut,
        Self: Sized,
    {
        self.0.encode_raw(buf)
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: prost::encoding::WireType,
        buf: &mut B,
        ctx: prost::encoding::DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: prost::bytes::Buf,
        Self: Sized,
    {
        self.0.merge_field(tag, wire_type, buf, ctx)
    }

    fn encoded_len(&self) -> usize {
        self.0.encoded_len()
    }

    fn clear(&mut self) {
        self.0.clear()
    }
}

impl Default for SerdeTimestamp {
    fn default() -> Self {
        SerdeTimestamp(Timestamp::default())
    }
}

impl From<Timestamp> for SerdeTimestamp {
    fn from(ts: Timestamp) -> Self {
        SerdeTimestamp(ts)
    }
}

impl From<SerdeTimestamp> for Timestamp {
    fn from(st: SerdeTimestamp) -> Self {
        st.0
    }
}

impl From<SystemTime> for SerdeTimestamp {
    fn from(st: SystemTime) -> Self {
        let duration = st.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        let ts = Timestamp {
            seconds: duration.as_secs() as i64,
            nanos: duration.subsec_nanos() as i32,
        };
        SerdeTimestamp(ts)
    }
}

impl SerdeTimestamp {
    pub fn now() -> Self {
        SystemTime::now().into()
    }
    
    pub fn to_system_time(&self) -> SystemTime {
        UNIX_EPOCH + Duration::new(self.0.seconds as u64, self.0.nanos as u32)
    }
    
    pub fn inner(&self) -> &Timestamp {
        &self.0
    }
    
    pub fn into_inner(self) -> Timestamp {
        self.0
    }
}

// Make it easier to work with the wrapper
impl std::ops::Deref for SerdeTimestamp {
    type Target = Timestamp;
    
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SerdeTimestamp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}