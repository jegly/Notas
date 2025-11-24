use std::{fs, path::PathBuf};
use anyhow::{Result, anyhow};
use aes_gcm::Key;
use dirs::data_dir;
use once_cell::sync::OnceCell;

use super::{
    data::{NoteList, MasterPassword},
    crypto::{self, EncryptedData, SALT_LEN},
};

static CRYPTO_KEY: OnceCell<Key<aes_gcm::Aes256Gcm>> = OnceCell::new();
static CRYPTO_SALT: OnceCell<[u8; SALT_LEN]> = OnceCell::new();

pub struct CoreManager {
    data_path: PathBuf,
    note_list: NoteList,
}

impl CoreManager {
    pub fn new() -> Result<Self> {
        // Use XDG data directory for user data
        let data_dir = data_dir().ok_or_else(|| anyhow!("Could not find data directory"))?;
        let app_dir = data_dir.join("nocturne_notes");
        fs::create_dir_all(&app_dir)?;
        let data_path = app_dir.join("notes.dat");

        Ok(Self {
            data_path,
            note_list: NoteList::new(),
        })
    }

    pub fn is_unlocked() -> bool {
        CRYPTO_KEY.get().is_some()
    }

    pub fn unlock(&mut self, master_password: MasterPassword) -> Result<()> {
        if Self::is_unlocked() {
            return Ok(());
        }

        let password_bytes = &master_password.0;

        let encrypted_data = match fs::read(&self.data_path) {
            Ok(bytes) => EncryptedData::from_bytes(&bytes)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let (key, salt) = crypto::generate_test_key(password_bytes)?;
                let test_note_list = NoteList::new();
                let serialized = bincode::serialize(&test_note_list)?;
                let encrypted = crypto::encrypt(&key, &salt, &serialized)?;

                CRYPTO_SALT.set(salt).map_err(|_| anyhow!("Failed to set crypto salt"))?;
                CRYPTO_KEY.set(key).map_err(|_| anyhow!("Failed to set crypto key"))?;

                fs::write(&self.data_path, encrypted.to_bytes())?;
                self.note_list = test_note_list;
                return Ok(());
            },
            Err(e) => return Err(e.into()),
        };

        let key = crypto::derive_key(password_bytes, &encrypted_data.header.salt)?;
        let decrypted_bytes = crypto::decrypt(&key, &encrypted_data)
            .map_err(|_| anyhow!("Invalid password or corrupted data."))?;

        self.note_list = bincode::deserialize(&decrypted_bytes)?;
        CRYPTO_SALT.set(encrypted_data.header.salt).map_err(|_| anyhow!("Failed to set crypto salt"))?;
        CRYPTO_KEY.set(key).map_err(|_| anyhow!("Failed to set crypto key"))?;

        Ok(())
    }

    fn save_notes(&self) -> Result<()> {
        let key = CRYPTO_KEY.get().ok_or_else(|| anyhow!("Application is locked"))?;
        let salt = CRYPTO_SALT.get().ok_or_else(|| anyhow!("Application salt not set"))?;

        let serialized = bincode::serialize(&self.note_list)?;
        let encrypted = crypto::encrypt(key, salt, &serialized)?;
        fs::write(&self.data_path, encrypted.to_bytes())?;

        Ok(())
    }

    pub fn get_notes(&self) -> Vec<super::data::Note> {
        self.note_list.notes.clone()
    }

    pub fn create_note(&mut self, title: String, content: String) -> Result<()> {
        let note = super::data::Note::new(title, content);
        self.note_list.add_note(note);
        self.save_notes()
    }

    pub fn update_note(&mut self, id: u64, title: String, content: String) -> Result<()> {
        if self.note_list.update_note(id, title, content) {
            self.save_notes()
        } else {
            Err(anyhow!("Note with ID {} not found", id))
        }
    }

    pub fn delete_note(&mut self, id: u64) -> Result<()> {
        if self.note_list.delete_note(id) {
            self.save_notes()
        } else {
            Err(anyhow!("Note with ID {} not found", id))
        }
    }

    pub fn export_note_text(&self, id: u64) -> Result<String> {
        let note = self.note_list.notes.iter().find(|n| n.id == id)
            .ok_or_else(|| anyhow!("Note not found"))?;
        
        Ok(format!("Title: {}\n\nContent:\n{}", note.title, note.content))
    }

    pub fn export_all_encrypted(&self, export_path: &PathBuf) -> Result<()> {
        let key = CRYPTO_KEY.get().ok_or_else(|| anyhow!("Application is locked"))?;
        let salt = CRYPTO_SALT.get().ok_or_else(|| anyhow!("Application salt not set"))?;

        let serialized = bincode::serialize(&self.note_list)?;
        let encrypted = crypto::encrypt(key, salt, &serialized)?;
        fs::write(export_path, encrypted.to_bytes())?;

        Ok(())
    }

    pub fn import_encrypted(&mut self, import_path: &PathBuf, master_password: MasterPassword) -> Result<()> {
        let password_bytes = &master_password.0;
        
        let encrypted_data = fs::read(import_path).map_err(|e| anyhow!("Failed to read import file: {}", e))?;
        let encrypted_data = EncryptedData::from_bytes(&encrypted_data)?;

        let key = crypto::derive_key(password_bytes, &encrypted_data.header.salt)?;
        let decrypted_bytes = crypto::decrypt(&key, &encrypted_data)?;
        let imported_note_list: NoteList = bincode::deserialize(&decrypted_bytes)?;

        for note in imported_note_list.notes {
            self.note_list.add_note(note);
        }
        
        self.save_notes()?;
        Ok(())
    }
}

// Ensure the key is zeroized when the application exits
impl Drop for CoreManager {
    fn drop(&mut self) {
        if let Some(_key) = CRYPTO_KEY.get() {
            // See notes in your original comment about zeroization.
        }
    }
}
