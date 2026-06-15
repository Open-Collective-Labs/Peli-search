use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::document::Document;
use crate::schema::Mapping;

/// A single entry in the write-ahead log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum WalEntry {
    /// Create a new named index with the given mapping.
    CreateIndex {
        name: String,
        mapping: Mapping,
    },
    /// Delete an index and all its data.
    DeleteIndex {
        name: String,
    },
    /// Add a document to an index.
    AddDocument {
        index_name: String,
        document: Document,
    },
    /// Remove a document from an index.
    RemoveDocument {
        index_name: String,
        doc_id: String,
    },
}

/// An append-only write-ahead log for durability.
///
/// Each entry is serialized as a JSON line. The write flow is:
///
/// 1. `append()` — write the entry as a line in the log buffer
/// 2. `flush()` — force the buffer to disk (fsync)
/// 3. Apply the operation to in-memory state
///
/// This ordering guarantees that on recovery, any operation reflected in
/// memory can be recovered from the WAL.
pub struct Wal {
    path: PathBuf,
    file: Option<File>,
    buf: io::BufWriter<File>,
    entry_count: u64,
}

impl Wal {
    /// Open or create a WAL at the given path.
    ///
    /// If the file already exists, it is opened in append mode and existing
    /// entries are counted.
    pub fn open(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;

        let existing_count = count_valid_entries(&file)?;
        // Seek to end for appending (file is opened in append mode, but let's
        // ensure the BufWriter starts at the end).
        let file_clone = file.try_clone()?;
        let buf = io::BufWriter::new(file);

        Ok(Self {
            path,
            file: Some(file_clone),
            buf,
            entry_count: existing_count,
        })
    }

    /// Append a single entry to the WAL buffer.
    ///
    /// The entry is serialized to JSON and written as a newline-terminated line.
    /// Data is NOT flushed to disk — call `flush()` to guarantee durability.
    pub fn append(&mut self, entry: &WalEntry) -> io::Result<()> {
        let line = serde_json::to_string(entry).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("serialization error: {e}"))
        })?;
        writeln!(self.buf, "{line}")?;
        self.entry_count += 1;
        Ok(())
    }

    /// Force all buffered WAL data to disk.
    ///
    /// Calls `flush()` on the `BufWriter` and `sync_all()` on the underlying
    /// file to guarantee durability. After this returns successfully, the
    /// entry is recoverable after a crash.
    pub fn flush(&mut self) -> io::Result<()> {
        self.buf.flush()?;
        if let Some(file) = self.file.as_ref() {
            file.sync_all()?;
        }
        Ok(())
    }

    /// Replay all valid entries stored in the WAL.
    ///
    /// Corrupted or partially-written trailing lines are skipped gracefully
    /// (logical corruption in the middle of the file is not expected under
    /// normal operation, but trailing truncation from a crash is handled).
    ///
    /// Returns the recovered entries in order.
    pub fn replay(&self) -> io::Result<Vec<WalEntry>> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line_result in reader.lines() {
            let line = match line_result {
                Ok(l) => l,
                Err(_) => {
                    // I/O error reading a line — treat as end of recoverable data
                    break;
                }
            };

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_str::<WalEntry>(trimmed) {
                Ok(entry) => entries.push(entry),
                Err(_) => {
                    // Corrupt line — this is typically a partially-written
                    // trailing line from a crash. Stop replay here since all
                    // subsequent data is unreliable (lines are written
                    // sequentially, so corruption can only affect the tail).
                    break;
                }
            }
        }

        Ok(entries)
    }

    /// Truncate the WAL, discarding all entries.
    ///
    /// This is called after a successful snapshot to reset the log.
    pub fn truncate(&mut self) -> io::Result<()> {
        // Open a new file, truncating the existing one
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&self.path)?;
        let file_clone = file.try_clone()?;
        self.buf = io::BufWriter::new(file);
        self.file = Some(file_clone);
        self.entry_count = 0;
        Ok(())
    }

    /// Return the number of entries written since open or last truncation.
    pub fn entry_count(&self) -> u64 {
        self.entry_count
    }

    /// Return the path to the WAL file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Count the number of valid, non-empty JSON lines in the file.
fn count_valid_entries(file: &File) -> io::Result<u64> {
    let reader = BufReader::new(file.try_clone()?);
    let mut count = 0u64;
    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_entry() -> WalEntry {
        WalEntry::CreateIndex {
            name: "test".into(),
            mapping: Mapping::new(vec![]),
        }
    }

    #[test]
    fn append_and_replay() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wal.log");

        let mut wal = Wal::open(&path).unwrap();
        assert_eq!(wal.entry_count(), 0);

        wal.append(&create_entry()).unwrap();
        assert_eq!(wal.entry_count(), 1);

        wal.append(&WalEntry::DeleteIndex { name: "old".into() })
            .unwrap();
        assert_eq!(wal.entry_count(), 2);

        // Flush after writes to ensure durability
        wal.flush().unwrap();

        let entries = wal.replay().unwrap();
        assert_eq!(entries.len(), 2);
        assert!(matches!(entries[0], WalEntry::CreateIndex { ref name, .. } if name == "test"));
        assert!(matches!(entries[1], WalEntry::DeleteIndex { ref name, .. } if name == "old"));
    }

    #[test]
    fn append_separate_from_flush() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wal.log");

        let mut wal = Wal::open(&path).unwrap();
        wal.append(&create_entry()).unwrap();
        // Data is buffered but not yet flushed — still counted
        assert_eq!(wal.entry_count(), 1);
        // Explicit flush
        wal.flush().unwrap();

        let entries = wal.replay().unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn flush_ensures_recoverability() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wal.log");

        // Write entries with flush, then drop the WAL
        {
            let mut wal = Wal::open(&path).unwrap();
            wal.append(&create_entry()).unwrap();
            wal.flush().unwrap();
        }

        // Re-open and verify entries survived
        let wal = Wal::open(&path).unwrap();
        assert_eq!(wal.entry_count(), 1);

        let entries = wal.replay().unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn without_flush_data_may_be_lost_on_crash() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wal.log");

        // Write entry but do NOT flush
        {
            let mut wal = Wal::open(&path).unwrap();
            wal.append(&create_entry()).unwrap();
            // Drop without flush simulates a crash
        }

        // On reopen, entry_count reflects what's on disk
        let wal = Wal::open(&path).unwrap();
        // The data might or might not be on disk without flush.
        // What matters is that replay returns the entries that are on disk.
        let entries = wal.replay().unwrap();
        // Entry may or may not be present — but won't cause errors
        assert!(entries.len() <= 1);
    }

    #[test]
    fn corrupt_trailing_line_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wal.log");

        // Write two valid entries using our WAL
        {
            let mut wal = Wal::open(&path).unwrap();
            wal.append(&create_entry()).unwrap();
            wal.append(&WalEntry::DeleteIndex { name: "old".into() }).unwrap();
            wal.flush().unwrap();
        }

        // Append a corrupt line directly to the file (simulating crash during write)
        {
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            writeln!(file, "{{corrupt_json").unwrap();
            // No flush — partial write
        }

        // Replay should recover the valid entries and stop at the corrupt line
        let wal = Wal::open(&path).unwrap();
        let entries = wal.replay().unwrap();
        assert_eq!(entries.len(), 2, "should recover 2 valid entries, skipping corrupt tail");
    }

    #[test]
    fn corrupt_middle_line_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wal.log");

        // Write: valid, corrupt, valid
        {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)
                .unwrap();

            let e1 = create_entry();
            writeln!(file, "{}", serde_json::to_string(&e1).unwrap()).unwrap();

            writeln!(file, "{{not_valid_json}}").unwrap();

            let e2 = WalEntry::DeleteIndex { name: "old".into() };
            writeln!(file, "{}", serde_json::to_string(&e2).unwrap()).unwrap();

            file.flush().unwrap();
        }

        // Replay — the corrupt line in the middle breaks the chain
        // since lines are written sequentially, corruption in the middle
        // makes all subsequent data unreliable.
        let wal = Wal::open(&path).unwrap();
        let entries = wal.replay().unwrap();
        // We expect only 1 valid entry before the corruption
        assert_eq!(entries.len(), 1, "replay stops at corrupt line, subsequent data is unreliable");
    }

    #[test]
    fn truncate_clears_log() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wal.log");

        let mut wal = Wal::open(&path).unwrap();
        wal.append(&create_entry()).unwrap();
        wal.flush().unwrap();
        assert_eq!(wal.entry_count(), 1);

        wal.truncate().unwrap();
        assert_eq!(wal.entry_count(), 0);

        let entries = wal.replay().unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn append_after_truncate() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wal.log");

        let mut wal = Wal::open(&path).unwrap();
        wal.append(&create_entry()).unwrap();
        wal.flush().unwrap();
        wal.truncate().unwrap();
        wal.append(&create_entry()).unwrap();
        wal.flush().unwrap();

        let entries = wal.replay().unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn add_document_entry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wal.log");

        let mut wal = Wal::open(&path).unwrap();
        let mut fields = std::collections::HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("hello"));
        let doc = Document::new("doc1", fields).unwrap();

        wal.append(&WalEntry::AddDocument {
            index_name: "idx".into(),
            document: doc.clone(),
        })
        .unwrap();
        wal.flush().unwrap();

        let entries = wal.replay().unwrap();
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            WalEntry::AddDocument {
                index_name,
                document,
            } => {
                assert_eq!(index_name, "idx");
                assert_eq!(document.id, "doc1");
            }
            _ => panic!("expected AddDocument"),
        }
    }

    #[test]
    fn remove_document_entry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wal.log");

        let mut wal = Wal::open(&path).unwrap();
        wal.append(&WalEntry::RemoveDocument {
            index_name: "idx".into(),
            doc_id: "doc1".into(),
        })
        .unwrap();
        wal.flush().unwrap();

        let entries = wal.replay().unwrap();
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            WalEntry::RemoveDocument { index_name, doc_id } => {
                assert_eq!(index_name, "idx");
                assert_eq!(doc_id, "doc1");
            }
            _ => panic!("expected RemoveDocument"),
        }
    }

    #[test]
    fn replay_empty_log() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.log");

        let wal = Wal::open(&path).unwrap();
        let entries = wal.replay().unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn serde_roundtrip_all_variants() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wal.log");

        let mut wal = Wal::open(&path).unwrap();

        let entries = vec![
            WalEntry::CreateIndex {
                name: "a".into(),
                mapping: Mapping::new(vec![]),
            },
            WalEntry::DeleteIndex { name: "b".into() },
            WalEntry::AddDocument {
                index_name: "c".into(),
                document: Document::new("d1", std::collections::HashMap::new()).unwrap(),
            },
            WalEntry::RemoveDocument {
                index_name: "d".into(),
                doc_id: "d1".into(),
            },
        ];

        for e in &entries {
            wal.append(e).unwrap();
        }
        wal.flush().unwrap();

        let replayed = wal.replay().unwrap();
        assert_eq!(replayed, entries);
    }

    #[test]
    fn count_matches_replayed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wal.log");

        let mut wal = Wal::open(&path).unwrap();
        wal.append(&create_entry()).unwrap();
        wal.append(&create_entry()).unwrap();
        wal.append(&create_entry()).unwrap();
        wal.flush().unwrap();

        // entry_count should match the number of replayed entries
        assert_eq!(wal.entry_count(), 3);

        let entries = wal.replay().unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn survives_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wal.log");

        // First session
        let entry_count = {
            let mut wal = Wal::open(&path).unwrap();
            wal.append(&create_entry()).unwrap();
            wal.flush().unwrap();
            wal.entry_count()
        };
        assert_eq!(entry_count, 1);

        // Second session — open same file
        let wal = Wal::open(&path).unwrap();
        assert_eq!(wal.entry_count(), 1);

        let entries = wal.replay().unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn partial_line_at_end_handled() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wal.log");

        // Write one full line followed by a truncated line (no newline)
        {
            let e = create_entry();
            let json = serde_json::to_string(&e).unwrap();
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)
                .unwrap();
            write!(file, "{json}\n").unwrap();
            // Write a partial JSON line (simulating crash mid-write)
            write!(file, "{{\"op\":\"AddD").unwrap();
            file.flush().unwrap();
        }

        // Replay should recover only the complete entry
        let wal = Wal::open(&path).unwrap();
        let entries = wal.replay().unwrap();
        assert_eq!(entries.len(), 1, "partial trailing line should be skipped");
    }

    #[test]
    fn only_whitespace_lines_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wal.log");

        {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)
                .unwrap();
            writeln!(file, "   ").unwrap();
            let e = create_entry();
            writeln!(file, "{}", serde_json::to_string(&e).unwrap()).unwrap();
            writeln!(file, "").unwrap();
            file.flush().unwrap();
        }

        let wal = Wal::open(&path).unwrap();
        let entries = wal.replay().unwrap();
        assert_eq!(entries.len(), 1);
    }
}
