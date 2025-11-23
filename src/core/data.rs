use serde::{Serialize, Deserialize};
use chrono::{Utc, DateTime};
use zeroize::Zeroize;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Note {
    pub id: u64,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Note {
    pub fn new(title: String, content: String) -> Self {
        let now = Utc::now();
        Self {
            id: now.timestamp_millis() as u64, // Simple ID generation
            title,
            content,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NoteList {
    pub notes: Vec<Note>,
}

impl NoteList {
    pub fn new() -> Self {
        Self { notes: Vec::new() }
    }

    pub fn add_note(&mut self, note: Note) {
        self.notes.push(note);
        self.notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    }

    pub fn delete_note(&mut self, id: u64) -> bool {
        let initial_len = self.notes.len();
        self.notes.retain(|note| note.id != id);
        self.notes.len() < initial_len
    }

    pub fn update_note(&mut self, id: u64, title: String, content: String) -> bool {
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == id) {
            note.title = title;
            note.content = content;
            note.updated_at = Utc::now();
            self.notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            true
        } else {
            false
        }
    }
}

// Struct to hold the master password securely in memory
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct MasterPassword(pub Vec<u8>);

impl From<String> for MasterPassword {
    fn from(s: String) -> Self {
        MasterPassword(s.into_bytes())
    }
}

impl From<&str> for MasterPassword {
    fn from(s: &str) -> Self {
        MasterPassword(s.as_bytes().to_vec())
    }
}
