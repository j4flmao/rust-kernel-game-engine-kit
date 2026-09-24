//! Bounded offline-first state replication primitives.

use std::{
    collections::VecDeque,
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use hmac::{Hmac, Mac};
use sha2::Sha256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncMode {
    Offline,
    Online,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncKind {
    Snapshot,
    Delta,
    Event,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncConfig {
    pub max_pending: usize,
    pub max_payload_bytes: usize,
    pub max_in_flight: usize,
    pub resend_after_frames: u64,
    pub max_retries: u8,
}
impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            max_pending: 4096,
            max_payload_bytes: 256 * 1024,
            max_in_flight: 128,
            resend_after_frames: 3,
            max_retries: 8,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncPacket {
    pub sequence: u64,
    pub ack: u64,
    pub kind: SyncKind,
    pub key: u64,
    pub payload: Vec<u8>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncError {
    EmptyPayload,
    PayloadTooLarge,
    Backpressure,
    AllocationFailed,
    InvalidPacket,
    ReceiveTooLarge,
    PersistenceFailed,
    InvalidConfig,
    NoSession,
    AuthenticationFailed,
}

/// Authentication hook supplied by the transport layer.
///
/// The kernel intentionally does not ship cryptography. Implement this with
/// an audited HMAC/AEAD library in the networking adapter; `message` includes
/// the session nonce and the complete encoded packet.
pub trait SyncAuthenticator {
    fn authentication_tag(&self, message: &[u8]) -> [u8; 32];

    fn verify_tag(&self, message: &[u8], expected: &[u8; 32]) -> bool {
        let actual = self.authentication_tag(message);
        let mut different = 0u8;
        for (left, right) in actual.iter().zip(expected.iter()) {
            different |= left ^ right;
        }
        different == 0
    }
}

type HmacSha256 = Hmac<Sha256>;

/// Production authentication adapter using HMAC-SHA256 from RustCrypto.
///
/// This authenticates integrity and origin. It does not provide secrecy; use
/// an AEAD transport when packet confidentiality is required.
pub struct HmacSha256Authenticator {
    key: Vec<u8>,
}

impl HmacSha256Authenticator {
    pub fn new(key: &[u8]) -> Result<Self, SyncError> {
        if key.is_empty() {
            return Err(SyncError::AuthenticationFailed);
        }
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(key.len())
            .map_err(|_| SyncError::AllocationFailed)?;
        owned.extend_from_slice(key);
        Ok(Self { key: owned })
    }
}

impl SyncAuthenticator for HmacSha256Authenticator {
    fn authentication_tag(&self, message: &[u8]) -> [u8; 32] {
        let mut mac = HmacSha256::new_from_slice(&self.key)
            .expect("HMAC accepts keys of every non-zero length");
        mac.update(message);
        let bytes = mac.finalize().into_bytes();
        let mut tag = [0u8; 32];
        tag.copy_from_slice(&bytes);
        tag
    }
}

impl Drop for HmacSha256Authenticator {
    fn drop(&mut self) {
        self.key.fill(0);
    }
}

#[derive(Debug)]
pub enum WalError {
    Io(io::Error),
    InvalidRecord,
    RecordTooLarge,
    TooManyRecords,
    AllocationFailed,
}
impl core::fmt::Display for WalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "sync WAL I/O failed: {error}"),
            Self::InvalidRecord => f.write_str("invalid sync WAL record"),
            Self::RecordTooLarge => f.write_str("sync WAL record exceeds configured bound"),
            Self::TooManyRecords => f.write_str("sync WAL exceeds configured record bound"),
            Self::AllocationFailed => f.write_str("sync WAL allocation failed"),
        }
    }
}
impl core::error::Error for WalError {}
impl From<io::Error> for WalError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Crash-tolerant append-only queue for offline packets.
///
/// Each record is length-checked and checksummed. A torn final record is
/// ignored (the expected result of a process or power loss during append),
/// while a corrupt complete record fails closed. Authentication is deliberately
/// not claimed here; callers still need an authenticated transport.
pub struct SyncWal {
    path: PathBuf,
    max_payload_bytes: usize,
    max_records: usize,
}
impl SyncWal {
    const MAGIC: [u8; 4] = *b"RKW1";
    const PACKET: u8 = 1;
    const ACK: u8 = 2;
    const HEADER: usize = 4 + 1 + 4;
    const TRAILER: usize = 4;

    pub fn open(path: impl AsRef<Path>, max_payload_bytes: usize) -> Result<Self, WalError> {
        Self::open_with_limits(path, max_payload_bytes, 4096)
    }

    pub fn open_with_limits(
        path: impl AsRef<Path>,
        max_payload_bytes: usize,
        max_records: usize,
    ) -> Result<Self, WalError> {
        if max_records == 0 {
            return Err(WalError::TooManyRecords);
        }
        let path = path.as_ref().to_path_buf();
        OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;
        Ok(Self {
            path,
            max_payload_bytes,
            max_records,
        })
    }

    pub fn append(&self, packet: &SyncPacket) -> Result<(), WalError> {
        if packet.payload.is_empty() || packet.payload.len() > self.max_payload_bytes {
            return Err(WalError::RecordTooLarge);
        }
        let body = Replicator::encode_packet(packet).map_err(|error| match error {
            SyncError::AllocationFailed => WalError::AllocationFailed,
            _ => WalError::RecordTooLarge,
        })?;
        self.append_record(Self::PACKET, &body)
    }

    pub fn acknowledge(&self, sequence: u64) -> Result<(), WalError> {
        self.append_record(Self::ACK, &sequence.to_le_bytes())
    }

    pub fn replay(&self) -> Result<Vec<SyncPacket>, WalError> {
        let mut file = File::open(&self.path)?;
        let mut pending: Vec<SyncPacket> = Vec::new();
        loop {
            let mut header = [0u8; Self::HEADER];
            match file.read_exact(&mut header) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(error.into()),
            }
            if header[..4] != Self::MAGIC {
                return Err(WalError::InvalidRecord);
            }
            let kind = header[4];
            let len = u32::from_le_bytes(
                header[5..9]
                    .try_into()
                    .map_err(|_| WalError::InvalidRecord)?,
            ) as usize;
            let max_record = self
                .max_payload_bytes
                .checked_add(30)
                .ok_or(WalError::RecordTooLarge)?;
            if len == 0 || len > max_record {
                return Err(WalError::RecordTooLarge);
            }
            let mut body = Vec::new();
            body.try_reserve_exact(len)
                .map_err(|_| WalError::AllocationFailed)?;
            body.resize(len, 0);
            match file.read_exact(&mut body) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(error.into()),
            }
            let mut checksum = [0u8; Self::TRAILER];
            match file.read_exact(&mut checksum) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(error.into()),
            }
            let expected = u32::from_le_bytes(checksum);
            if wal_checksum(kind, &body) != expected {
                return Err(WalError::InvalidRecord);
            }
            match kind {
                Self::PACKET => {
                    let packet = decode_packet(&body, self.max_payload_bytes)?;
                    if let Some(existing) =
                        pending.iter_mut().find(|p| p.sequence == packet.sequence)
                    {
                        *existing = packet;
                    } else {
                        if pending.len() >= self.max_records {
                            return Err(WalError::TooManyRecords);
                        }
                        pending.push(packet);
                    }
                }
                Self::ACK => {
                    if len != 8 {
                        return Err(WalError::InvalidRecord);
                    }
                    let sequence =
                        u64::from_le_bytes(body.try_into().map_err(|_| WalError::InvalidRecord)?);
                    pending.retain(|packet| packet.sequence > sequence);
                }
                _ => return Err(WalError::InvalidRecord),
            }
        }
        Ok(pending)
    }

    /// Rewrites the live pending set into a compact WAL. The temporary file
    /// is synced before replacement, so a failed write does not corrupt the
    /// original log. Call this after acknowledged records have accumulated.
    pub fn compact(&self, packets: &[SyncPacket]) -> Result<(), WalError> {
        if packets.len() > self.max_records {
            return Err(WalError::TooManyRecords);
        }
        let temp_path = self.path.with_extension("compact");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temp_path)?;
        for packet in packets {
            if packet.payload.is_empty() || packet.payload.len() > self.max_payload_bytes {
                return Err(WalError::RecordTooLarge);
            }
            let body = Replicator::encode_packet(packet).map_err(|error| match error {
                SyncError::AllocationFailed => WalError::AllocationFailed,
                _ => WalError::RecordTooLarge,
            })?;
            write_wal_record(&mut file, Self::PACKET, &body)?;
        }
        file.sync_all()?;
        drop(file);
        if let Err(error) = std::fs::rename(&temp_path, &self.path) {
            #[cfg(windows)]
            {
                if error.kind() == io::ErrorKind::AlreadyExists {
                    std::fs::remove_file(&self.path)?;
                    std::fs::rename(&temp_path, &self.path)?;
                } else {
                    return Err(error.into());
                }
            }
            #[cfg(not(windows))]
            return Err(error.into());
        }
        Ok(())
    }

    fn append_record(&self, kind: u8, body: &[u8]) -> Result<(), WalError> {
        if body.is_empty() || body.len() > u32::MAX as usize {
            return Err(WalError::RecordTooLarge);
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        write_wal_record(&mut file, kind, body)?;
        file.sync_data()?;
        Ok(())
    }
}

fn write_wal_record(file: &mut File, kind: u8, body: &[u8]) -> Result<(), WalError> {
    if body.is_empty() || body.len() > u32::MAX as usize {
        return Err(WalError::RecordTooLarge);
    }
    file.write_all(&SyncWal::MAGIC)?;
    file.write_all(&[kind])?;
    file.write_all(&(body.len() as u32).to_le_bytes())?;
    file.write_all(body)?;
    file.write_all(&wal_checksum(kind, body).to_le_bytes())?;
    Ok(())
}

fn wal_checksum(kind: u8, body: &[u8]) -> u32 {
    let mut hash = 2_166_136_261u32 ^ u32::from(kind);
    for byte in body {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}

fn decode_packet(bytes: &[u8], max_payload_bytes: usize) -> Result<SyncPacket, WalError> {
    if bytes.len() < 30 || bytes[0] != 1 {
        return Err(WalError::InvalidRecord);
    }
    let kind = match bytes[1] {
        0 => SyncKind::Snapshot,
        1 => SyncKind::Delta,
        2 => SyncKind::Event,
        _ => return Err(WalError::InvalidRecord),
    };
    let sequence = u64::from_le_bytes(
        bytes[2..10]
            .try_into()
            .map_err(|_| WalError::InvalidRecord)?,
    );
    let ack = u64::from_le_bytes(
        bytes[10..18]
            .try_into()
            .map_err(|_| WalError::InvalidRecord)?,
    );
    let key = u64::from_le_bytes(
        bytes[18..26]
            .try_into()
            .map_err(|_| WalError::InvalidRecord)?,
    );
    let len = u32::from_le_bytes(
        bytes[26..30]
            .try_into()
            .map_err(|_| WalError::InvalidRecord)?,
    ) as usize;
    let total = 30usize.checked_add(len).ok_or(WalError::RecordTooLarge)?;
    if len == 0 || len > max_payload_bytes || total != bytes.len() {
        return Err(WalError::RecordTooLarge);
    }
    let mut payload = Vec::new();
    payload
        .try_reserve_exact(len)
        .map_err(|_| WalError::AllocationFailed)?;
    payload.extend_from_slice(&bytes[30..]);
    Ok(SyncPacket {
        sequence,
        ack,
        kind,
        key,
        payload,
    })
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SyncStats {
    pub queued: u64,
    pub coalesced: u64,
    pub sent: u64,
    pub resent: u64,
    pub acknowledged: u64,
    pub duplicates: u64,
    pub dropped_retries: u64,
    pub rejected_backpressure: u64,
    pub rejected_receive: u64,
}
#[derive(Clone, Debug)]
struct InFlight {
    packet: SyncPacket,
    last_sent: u64,
    retries: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportError {
    WouldBlock,
    Disconnected,
    Failed,
}
pub trait SyncTransport {
    fn send(&mut self, packet: &SyncPacket) -> Result<(), TransportError>;
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PumpReport {
    pub sent: usize,
    pub resent: usize,
    pub blocked: bool,
    pub failed: usize,
}

pub struct Replicator {
    config: SyncConfig,
    mode: SyncMode,
    next_sequence: u64,
    last_received: u64,
    last_ack_sent: u64,
    pending: VecDeque<SyncPacket>,
    in_flight: VecDeque<InFlight>,
    wal: Option<SyncWal>,
    session_nonce: Option<[u8; 16]>,
    stats: SyncStats,
}
impl Replicator {
    pub fn new(config: SyncConfig) -> Self {
        Self::try_new(config).expect("invalid sync configuration or allocation failure")
    }
    pub fn try_new(config: SyncConfig) -> Result<Self, SyncError> {
        if config.max_pending == 0 || config.max_payload_bytes == 0 || config.max_in_flight == 0 {
            return Err(SyncError::InvalidConfig);
        }
        let mut pending = VecDeque::new();
        pending
            .try_reserve_exact(config.max_pending)
            .map_err(|_| SyncError::AllocationFailed)?;
        let mut in_flight = VecDeque::new();
        in_flight
            .try_reserve_exact(config.max_in_flight)
            .map_err(|_| SyncError::AllocationFailed)?;
        Ok(Self {
            config,
            mode: SyncMode::Offline,
            next_sequence: 1,
            last_received: 0,
            last_ack_sent: 0,
            pending,
            in_flight,
            wal: None,
            session_nonce: None,
            stats: SyncStats::default(),
        })
    }
    pub fn with_wal(config: SyncConfig, wal: SyncWal) -> Result<Self, SyncError> {
        let mut replicator = Self::try_new(config)?;
        let recovered = wal.replay().map_err(|_| SyncError::PersistenceFailed)?;
        if recovered.len() > config.max_pending {
            return Err(SyncError::Backpressure);
        }
        for packet in recovered {
            replicator.next_sequence = replicator
                .next_sequence
                .max(packet.sequence.saturating_add(1).max(1));
            replicator.pending.push_back(packet);
        }
        replicator.wal = Some(wal);
        Ok(replicator)
    }
    pub fn attach_wal(&mut self, wal: SyncWal) -> Result<(), SyncError> {
        let recovered = wal.replay().map_err(|_| SyncError::PersistenceFailed)?;
        if self.pending.len().saturating_add(recovered.len()) > self.config.max_pending {
            return Err(SyncError::Backpressure);
        }
        for packet in recovered {
            self.next_sequence = self
                .next_sequence
                .max(packet.sequence.saturating_add(1).max(1));
            self.pending.push_back(packet);
        }
        self.wal = Some(wal);
        Ok(())
    }
    pub fn config(&self) -> SyncConfig {
        self.config
    }
    pub fn set_session_nonce(&mut self, nonce: [u8; 16]) {
        self.session_nonce = Some(nonce);
        self.last_received = 0;
        self.last_ack_sent = 0;
    }

    pub fn clear_session(&mut self) {
        self.session_nonce = None;
        self.last_received = 0;
        self.last_ack_sent = 0;
    }

    pub fn session_nonce(&self) -> Option<[u8; 16]> {
        self.session_nonce
    }
    pub fn mode(&self) -> SyncMode {
        self.mode
    }
    pub fn set_mode(&mut self, mode: SyncMode) {
        self.mode = mode;
    }
    pub fn stats(&self) -> SyncStats {
        self.stats
    }
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
    pub fn in_flight_len(&self) -> usize {
        self.in_flight.len()
    }
    pub fn last_received(&self) -> u64 {
        self.last_received
    }
    pub fn enqueue_snapshot(&mut self, payload: Vec<u8>) -> Result<u64, SyncError> {
        self.enqueue(SyncKind::Snapshot, 0, payload)
    }
    pub fn enqueue_snapshot_bytes(&mut self, payload: &[u8]) -> Result<u64, SyncError> {
        self.enqueue_owned(SyncKind::Snapshot, 0, payload)
    }
    pub fn enqueue_delta(&mut self, key: u64, payload: Vec<u8>) -> Result<u64, SyncError> {
        self.validate(&payload)?;
        if let Some(existing) = self
            .pending
            .iter_mut()
            .find(|p| p.kind == SyncKind::Delta && p.key == key)
        {
            let replacement = SyncPacket {
                sequence: existing.sequence,
                ack: self.last_received,
                kind: SyncKind::Delta,
                key,
                payload: payload.clone(),
            };
            if let Some(wal) = &self.wal {
                wal.append(&replacement)
                    .map_err(|_| SyncError::PersistenceFailed)?;
            }
            existing.payload = payload;
            self.stats.coalesced += 1;
            return Ok(existing.sequence);
        }
        self.enqueue(SyncKind::Delta, key, payload)
    }
    pub fn enqueue_delta_bytes(&mut self, key: u64, payload: &[u8]) -> Result<u64, SyncError> {
        self.validate(payload)?;
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(payload.len())
            .map_err(|_| SyncError::AllocationFailed)?;
        owned.extend_from_slice(payload);
        self.enqueue_delta(key, owned)
    }
    pub fn enqueue_event(&mut self, key: u64, payload: Vec<u8>) -> Result<u64, SyncError> {
        self.enqueue(SyncKind::Event, key, payload)
    }
    pub fn enqueue_event_bytes(&mut self, key: u64, payload: &[u8]) -> Result<u64, SyncError> {
        self.enqueue_owned(SyncKind::Event, key, payload)
    }
    fn enqueue_owned(
        &mut self,
        kind: SyncKind,
        key: u64,
        payload: &[u8],
    ) -> Result<u64, SyncError> {
        self.validate(payload)?;
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(payload.len())
            .map_err(|_| SyncError::AllocationFailed)?;
        owned.extend_from_slice(payload);
        self.enqueue(kind, key, owned)
    }
    fn validate(&self, payload: &[u8]) -> Result<(), SyncError> {
        if payload.is_empty() {
            Err(SyncError::EmptyPayload)
        } else if payload.len() > self.config.max_payload_bytes {
            Err(SyncError::PayloadTooLarge)
        } else {
            Ok(())
        }
    }
    fn enqueue(&mut self, kind: SyncKind, key: u64, payload: Vec<u8>) -> Result<u64, SyncError> {
        self.validate(&payload)?;
        if self.pending.len() >= self.config.max_pending {
            self.stats.rejected_backpressure += 1;
            return Err(SyncError::Backpressure);
        }
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1).max(1);
        let packet = SyncPacket {
            sequence,
            ack: self.last_received,
            kind,
            key,
            payload,
        };
        if let Some(wal) = &self.wal {
            wal.append(&packet)
                .map_err(|_| SyncError::PersistenceFailed)?;
        }
        self.pending.push_back(packet);
        self.stats.queued += 1;
        Ok(sequence)
    }
    pub fn pump<T: SyncTransport>(&mut self, transport: &mut T, frame: u64) -> PumpReport {
        if self.mode != SyncMode::Online {
            return PumpReport::default();
        }
        let mut report = PumpReport::default();
        while self.in_flight.len() < self.config.max_in_flight {
            let Some(packet) = self.pending.front().cloned() else {
                break;
            };
            match transport.send(&packet) {
                Ok(()) => {
                    self.pending.pop_front();
                    self.in_flight.push_back(InFlight {
                        packet,
                        last_sent: frame,
                        retries: 0,
                    });
                    self.stats.sent += 1;
                    report.sent += 1;
                }
                Err(_) => {
                    report.blocked = true;
                    break;
                }
            }
        }
        for item in &mut self.in_flight {
            if frame.saturating_sub(item.last_sent) < self.config.resend_after_frames
                || item.retries >= self.config.max_retries
            {
                continue;
            }
            match transport.send(&item.packet) {
                Ok(()) => {
                    item.last_sent = frame;
                    item.retries += 1;
                    self.stats.resent += 1;
                    report.resent += 1;
                }
                Err(_) => report.blocked = true,
            }
        }
        let mut retained = VecDeque::new();
        while let Some(item) = self.in_flight.pop_front() {
            if item.retries >= self.config.max_retries
                && frame.saturating_sub(item.last_sent) >= self.config.resend_after_frames
            {
                self.stats.dropped_retries += 1;
                report.failed += 1;
            } else {
                retained.push_back(item);
            }
        }
        self.in_flight = retained;
        report
    }
    pub fn acknowledge(&mut self, sequence: u64) {
        let _ = self.try_acknowledge(sequence);
    }
    pub fn try_acknowledge(&mut self, sequence: u64) -> Result<(), SyncError> {
        if let Some(wal) = &self.wal {
            wal.acknowledge(sequence)
                .map_err(|_| SyncError::PersistenceFailed)?;
        }
        let mut retained = VecDeque::new();
        while let Some(item) = self.in_flight.pop_front() {
            if item.packet.sequence <= sequence {
                self.stats.acknowledged += 1;
            } else {
                retained.push_back(item);
            }
        }
        self.in_flight = retained;
        Ok(())
    }
    pub fn receive(&mut self, packet: SyncPacket) -> Option<SyncPacket> {
        if packet.payload.is_empty() || packet.payload.len() > self.config.max_payload_bytes {
            self.stats.rejected_receive += 1;
            return None;
        }
        if packet.sequence <= self.last_received {
            self.stats.duplicates += 1;
            return None;
        }
        self.last_received = packet.sequence;
        if packet.ack > self.last_ack_sent {
            self.acknowledge(packet.ack);
            self.last_ack_sent = packet.ack;
        }
        Some(packet)
    }

    /// Decodes the compact little-endian wire envelope only after checking
    /// the declared payload length. No attacker-controlled allocation occurs
    /// before the size limit is accepted.
    pub fn receive_bytes(&mut self, bytes: &[u8]) -> Result<Option<SyncPacket>, SyncError> {
        const HEADER: usize = 1 + 1 + 8 + 8 + 8 + 4;
        if bytes.len() < HEADER {
            self.stats.rejected_receive += 1;
            return Err(SyncError::InvalidPacket);
        }
        let version = bytes[0];
        let kind = match bytes[1] {
            0 => SyncKind::Snapshot,
            1 => SyncKind::Delta,
            2 => SyncKind::Event,
            _ => {
                self.stats.rejected_receive += 1;
                return Err(SyncError::InvalidPacket);
            }
        };
        if version != 1 {
            self.stats.rejected_receive += 1;
            return Err(SyncError::InvalidPacket);
        }
        let mut sequence_bytes = [0; 8];
        sequence_bytes.copy_from_slice(&bytes[2..10]);
        let sequence = u64::from_le_bytes(sequence_bytes);
        let mut ack_bytes = [0; 8];
        ack_bytes.copy_from_slice(&bytes[10..18]);
        let ack = u64::from_le_bytes(ack_bytes);
        let mut key_bytes = [0; 8];
        key_bytes.copy_from_slice(&bytes[18..26]);
        let key = u64::from_le_bytes(key_bytes);
        let mut length_bytes = [0; 4];
        length_bytes.copy_from_slice(&bytes[26..30]);
        let len = u32::from_le_bytes(length_bytes) as usize;
        let total = HEADER.checked_add(len).ok_or(SyncError::ReceiveTooLarge)?;
        if len == 0 {
            self.stats.rejected_receive += 1;
            return Err(SyncError::EmptyPayload);
        }
        if len > self.config.max_payload_bytes || total != bytes.len() {
            self.stats.rejected_receive += 1;
            return Err(SyncError::ReceiveTooLarge);
        }
        let mut payload = Vec::new();
        payload
            .try_reserve_exact(len)
            .map_err(|_| SyncError::AllocationFailed)?;
        payload.extend_from_slice(&bytes[HEADER..]);
        Ok(self.receive(SyncPacket {
            sequence,
            ack,
            kind,
            key,
            payload,
        }))
    }

    pub fn encode_authenticated_packet<A: SyncAuthenticator>(
        packet: &SyncPacket,
        nonce: [u8; 16],
        authenticator: &A,
    ) -> Result<Vec<u8>, SyncError> {
        let encoded = Self::encode_packet(packet)?;
        let signed_len = 16usize
            .checked_add(encoded.len())
            .ok_or(SyncError::PayloadTooLarge)?;
        let total = signed_len
            .checked_add(32)
            .ok_or(SyncError::PayloadTooLarge)?;
        let mut signed = Vec::new();
        signed
            .try_reserve_exact(signed_len)
            .map_err(|_| SyncError::AllocationFailed)?;
        signed.extend_from_slice(&nonce);
        signed.extend_from_slice(&encoded);
        let tag = authenticator.authentication_tag(&signed);
        signed
            .try_reserve_exact(32)
            .map_err(|_| SyncError::AllocationFailed)?;
        signed.extend_from_slice(&tag);
        debug_assert_eq!(signed.len(), total);
        Ok(signed)
    }

    pub fn receive_authenticated_bytes<A: SyncAuthenticator>(
        &mut self,
        bytes: &[u8],
        authenticator: &A,
    ) -> Result<Option<SyncPacket>, SyncError> {
        const NONCE: usize = 16;
        const TAG: usize = 32;
        if bytes.len() < NONCE + TAG {
            self.stats.rejected_receive += 1;
            return Err(SyncError::AuthenticationFailed);
        }
        let configured = self.session_nonce.ok_or(SyncError::NoSession)?;
        let mut nonce = [0u8; NONCE];
        nonce.copy_from_slice(&bytes[..NONCE]);
        if nonce != configured {
            self.stats.rejected_receive += 1;
            return Err(SyncError::AuthenticationFailed);
        }
        let signed_end = bytes.len() - TAG;
        let mut tag = [0u8; TAG];
        tag.copy_from_slice(&bytes[signed_end..]);
        if !authenticator.verify_tag(&bytes[..signed_end], &tag) {
            self.stats.rejected_receive += 1;
            return Err(SyncError::AuthenticationFailed);
        }
        self.receive_bytes(&bytes[NONCE..signed_end])
    }

    pub fn encode_packet(packet: &SyncPacket) -> Result<Vec<u8>, SyncError> {
        if packet.payload.is_empty() {
            return Err(SyncError::EmptyPayload);
        }
        if packet.payload.len() > u32::MAX as usize {
            return Err(SyncError::PayloadTooLarge);
        }
        let mut out = Vec::new();
        out.try_reserve_exact(
            30usize
                .checked_add(packet.payload.len())
                .ok_or(SyncError::PayloadTooLarge)?,
        )
        .map_err(|_| SyncError::AllocationFailed)?;
        out.extend_from_slice(&[
            1,
            match packet.kind {
                SyncKind::Snapshot => 0,
                SyncKind::Delta => 1,
                SyncKind::Event => 2,
            },
        ]);
        out.extend_from_slice(&packet.sequence.to_le_bytes());
        out.extend_from_slice(&packet.ack.to_le_bytes());
        out.extend_from_slice(&packet.key.to_le_bytes());
        out.extend_from_slice(&(packet.payload.len() as u32).to_le_bytes());
        out.extend_from_slice(&packet.payload);
        Ok(out)
    }
}
impl Default for Replicator {
    fn default() -> Self {
        Self::new(SyncConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Sink;
    impl SyncTransport for Sink {
        fn send(&mut self, _: &SyncPacket) -> Result<(), TransportError> {
            Ok(())
        }
    }
    #[test]
    fn delta_coalesces() {
        let mut r = Replicator::new(SyncConfig {
            max_pending: 1,
            ..Default::default()
        });
        r.enqueue_delta(1, vec![1]).unwrap();
        r.enqueue_delta(1, vec![2]).unwrap();
        assert_eq!(r.pending_len(), 1);
        assert_eq!(r.stats().coalesced, 1);
    }
    #[test]
    fn backpressure_is_an_error() {
        let mut r = Replicator::new(SyncConfig {
            max_pending: 1,
            ..Default::default()
        });
        r.enqueue_event(1, vec![1]).unwrap();
        assert_eq!(r.enqueue_event(2, vec![2]), Err(SyncError::Backpressure));
    }
    #[test]
    fn online_ack_releases_inflight() {
        let mut r = Replicator::default();
        let mut sink = Sink;
        r.enqueue_event(1, vec![1]).unwrap();
        r.set_mode(SyncMode::Online);
        assert_eq!(r.pump(&mut sink, 1).sent, 1);
        r.acknowledge(1);
        assert_eq!(r.in_flight_len(), 0);
    }
    #[test]
    fn duplicate_is_suppressed() {
        let mut r = Replicator::default();
        let p = SyncPacket {
            sequence: 1,
            ack: 0,
            kind: SyncKind::Event,
            key: 1,
            payload: vec![1],
        };
        assert!(r.receive(p.clone()).is_some());
        assert!(r.receive(p).is_none());
    }
    #[test]
    fn wire_decoder_rejects_oversized_packet_before_allocation() {
        let mut r = Replicator::new(SyncConfig {
            max_payload_bytes: 4,
            ..Default::default()
        });
        let mut bytes = vec![1, 1];
        bytes.extend_from_slice(&1u64.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        bytes.extend_from_slice(&999u32.to_le_bytes());
        assert_eq!(r.receive_bytes(&bytes), Err(SyncError::ReceiveTooLarge));
        assert_eq!(r.stats().rejected_receive, 1);
    }

    #[test]
    fn wal_replays_packets_and_acknowledgements() {
        use std::{
            fs,
            time::{SystemTime, UNIX_EPOCH},
        };
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rust-kernel-sync-{nonce}.wal"));
        let wal = SyncWal::open(&path, 64).unwrap();
        wal.append(&SyncPacket {
            sequence: 1,
            ack: 0,
            kind: SyncKind::Event,
            key: 7,
            payload: vec![1, 2, 3],
        })
        .unwrap();
        wal.append(&SyncPacket {
            sequence: 2,
            ack: 0,
            kind: SyncKind::Event,
            key: 8,
            payload: vec![4],
        })
        .unwrap();
        wal.acknowledge(1).unwrap();
        let replayed = wal.replay().unwrap();
        assert_eq!(replayed.len(), 1);
        assert_eq!(replayed[0].sequence, 2);
        wal.compact(&replayed).unwrap();
        assert_eq!(wal.replay().unwrap(), replayed);
        drop(wal);
        let recovered =
            Replicator::with_wal(SyncConfig::default(), SyncWal::open(&path, 64).unwrap()).unwrap();
        assert_eq!(recovered.pending_len(), 1);
        assert_eq!(recovered.next_sequence, 3);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn authenticated_wire_requires_session_and_rejects_tampering() {
        let auth = HmacSha256Authenticator::new(b"test-only-key").unwrap();
        let packet = SyncPacket {
            sequence: 1,
            ack: 0,
            kind: SyncKind::Event,
            key: 9,
            payload: vec![7, 8],
        };
        let nonce = [3u8; 16];
        let bytes = Replicator::encode_authenticated_packet(&packet, nonce, &auth).unwrap();
        let mut receiver = Replicator::default();
        assert_eq!(
            receiver.receive_authenticated_bytes(&bytes, &auth),
            Err(SyncError::NoSession)
        );
        receiver.set_session_nonce(nonce);
        let mut tampered = bytes.clone();
        tampered[20] ^= 1;
        assert_eq!(
            receiver.receive_authenticated_bytes(&tampered, &auth),
            Err(SyncError::AuthenticationFailed)
        );
        assert_eq!(
            receiver.receive_authenticated_bytes(&bytes, &auth),
            Ok(Some(packet))
        );
    }
}
