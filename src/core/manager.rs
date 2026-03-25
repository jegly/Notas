use std::{fs, path::PathBuf};
use std::sync::RwLock;
use anyhow::{Result, anyhow};
use aes_gcm::Key;
use dirs::data_dir;
use once_cell::sync::OnceCell;
use zeroize::Zeroize;

use super::{
    data::{NoteList, MasterPassword, AppSettings, SecureBuffer, Argon2Params},
    crypto::{self, EncryptedData, SALT_LEN},
};

// Use RwLock for the crypto state to allow proper clearing
static CRYPTO_STATE: OnceCell<RwLock<Option<CryptoState>>> = OnceCell::new();

// Redirect file name - placed in default location to point to custom location
const REDIRECT_FILE: &str = "notes.redirect";

struct CryptoState {
    key: Key<aes_gcm::Aes256Gcm>,
    salt: [u8; SALT_LEN],
}

impl Zeroize for CryptoState {
    fn zeroize(&mut self) {
        // Key is 32 bytes, we need to zeroize the underlying data
        let key_bytes: &mut [u8; 32] = unsafe {
            &mut *(self.key.as_mut_ptr() as *mut [u8; 32])
        };
        key_bytes.zeroize();
        self.salt.zeroize();
    }
}

impl Drop for CryptoState {
    fn drop(&mut self) {
        self.zeroize();
    }
}

pub struct CoreManager {
    data_path: PathBuf,
    note_list: NoteList,
    settings: AppSettings,
}

impl CoreManager {
    pub fn new() -> Result<Self> {
        let mut settings = AppSettings::load();
        
        // Check for redirect file if no custom path is set in settings
        // This handles the case where settings.json was lost but redirect exists
        if settings.custom_db_path.is_none() {
            if let Some(redirect_path) = Self::check_redirect_file()? {
                settings.custom_db_path = Some(redirect_path);
                // Restore the settings file
                let _ = settings.save();
            }
        }
        
        let data_path = Self::resolve_data_path(&settings)?;

        Ok(Self {
            data_path,
            note_list: NoteList::new(),
            settings,
        })
    }
    
    /// Get the default app data directory
    fn get_default_app_dir() -> Result<PathBuf> {
        let data_dir = data_dir().ok_or_else(|| anyhow!("Could not find data directory"))?;
        let app_dir = data_dir.join("notas");
        fs::create_dir_all(&app_dir)?;
        Ok(app_dir)
    }
    
    /// Get the default database path
    fn get_default_db_path() -> Result<PathBuf> {
        Ok(Self::get_default_app_dir()?.join("notes.dat"))
    }
    
    /// Get the redirect file path
    fn get_redirect_path() -> Result<PathBuf> {
        Ok(Self::get_default_app_dir()?.join(REDIRECT_FILE))
    }
    
    /// Check if a redirect file exists and return the path it points to
    fn check_redirect_file() -> Result<Option<PathBuf>> {
        let redirect_path = Self::get_redirect_path()?;
        if redirect_path.exists() {
            let contents = fs::read_to_string(&redirect_path)?;
            let custom_path = PathBuf::from(contents.trim());
            // Only return if the custom path actually exists
            if custom_path.exists() {
                return Ok(Some(custom_path));
            }
        }
        Ok(None)
    }
    
    /// Write a redirect file pointing to the custom database location
    fn write_redirect_file(custom_path: &PathBuf) -> Result<()> {
        let redirect_path = Self::get_redirect_path()?;
        fs::write(&redirect_path, custom_path.display().to_string())?;
        Ok(())
    }
    
    /// Remove the redirect file (when moving back to default location)
    fn remove_redirect_file() -> Result<()> {
        let redirect_path = Self::get_redirect_path()?;
        if redirect_path.exists() {
            fs::remove_file(&redirect_path)?;
        }
        Ok(())
    }
    
    fn resolve_data_path(settings: &AppSettings) -> Result<PathBuf> {
        if let Some(ref custom_path) = settings.custom_db_path {
            if let Some(parent) = custom_path.parent() {
                fs::create_dir_all(parent)?;
            }
            Ok(custom_path.clone())
        } else {
            Self::get_default_db_path()
        }
    }
    
    pub fn get_data_path(&self) -> &PathBuf {
        &self.data_path
    }
    
    pub fn get_settings(&self) -> &AppSettings {
        &self.settings
    }
    
    pub fn update_settings(&mut self, settings: AppSettings) -> Result<()> {
        let new_path = Self::resolve_data_path(&settings)?;
        let path_changed = new_path != self.data_path;
        let old_path = self.data_path.clone();
        let was_custom = self.settings.custom_db_path.is_some();
        let is_custom = settings.custom_db_path.is_some();
        
        // Check if Argon2 params changed - need to re-encrypt
        let params_changed = self.settings.argon2_params.memory_cost != settings.argon2_params.memory_cost
            || self.settings.argon2_params.time_cost != settings.argon2_params.time_cost
            || self.settings.argon2_params.parallelism != settings.argon2_params.parallelism;
        
        if path_changed && Self::is_unlocked() {
            // Move data file to new location
            if old_path.exists() {
                if let Some(parent) = new_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&old_path, &new_path)?;
                fs::remove_file(&old_path)?;
            }
            self.data_path = new_path.clone();
            
            // Handle redirect file
            if is_custom {
                // Moving to custom location - create redirect file
                Self::write_redirect_file(&new_path)?;
            } else if was_custom {
                // Moving back to default - remove redirect file
                Self::remove_redirect_file()?;
            }
        } else if is_custom && !was_custom {
            // First time setting custom path (even if path didn't change)
            Self::write_redirect_file(&new_path)?;
        } else if !is_custom && was_custom {
            // Clearing custom path
            Self::remove_redirect_file()?;
        }
        
        self.settings = settings;
        self.settings.save()?;
        
        // If Argon2 params changed, we need user to re-enter password to re-encrypt
        // This is handled separately via re_encrypt_with_new_params
        if params_changed && Self::is_unlocked() {
            // Just save settings, user needs to call re_encrypt_with_new_params with password
        }
        
        Ok(())
    }
    
    /// Re-encrypt the vault with new Argon2 parameters
    #[allow(dead_code)]
    pub fn re_encrypt_with_params(&mut self, password: MasterPassword, new_params: &Argon2Params) -> Result<()> {
        if !Self::is_unlocked() {
            return Err(anyhow!("Application must be unlocked to change encryption parameters"));
        }
        
        let password_buffer = SecureBuffer::new(password.0.clone());
        let (new_key, new_salt) = crypto::generate_test_key_with_params(password_buffer.as_slice(), new_params)?;
        
        // Re-encrypt with new parameters
        let serialized = bincode::serialize(&self.note_list)?;
        let encrypted = crypto::encrypt(&new_key, &new_salt, &serialized)?;
        fs::write(&self.data_path, encrypted.to_bytes())?;
        
        // Update crypto state
        if let Some(state) = CRYPTO_STATE.get() {
            if let Ok(mut guard) = state.write() {
                if let Some(ref mut crypto) = *guard {
                    crypto.zeroize();
                }
                *guard = Some(CryptoState { 
                    key: new_key, 
                    salt: new_salt 
                });
            }
        }
        
        Ok(())
    }

    fn init_crypto_state() {
        let _ = CRYPTO_STATE.set(RwLock::new(None));
    }

    pub fn is_unlocked() -> bool {
        if let Some(state) = CRYPTO_STATE.get() {
            state.read().map(|s| s.is_some()).unwrap_or(false)
        } else {
            false
        }
    }
    
    pub fn lock(&mut self) {
        // Zeroize the note list in memory
        self.note_list.zeroize();
        self.note_list = NoteList::new();
        
        // Clear the crypto state
        if let Some(state) = CRYPTO_STATE.get() {
            if let Ok(mut guard) = state.write() {
                if let Some(ref mut crypto) = *guard {
                    crypto.zeroize();
                }
                *guard = None;
            }
        }
    }

    pub fn unlock(&mut self, master_password: MasterPassword) -> Result<()> {
        // Initialize crypto state container if needed
        Self::init_crypto_state();
        
        if Self::is_unlocked() {
            return Ok(());
        }

        // Use SecureBuffer to protect password in memory
        let password_buffer = SecureBuffer::new(master_password.0.clone());
        let password_bytes = password_buffer.as_slice();
        
        // ALWAYS use default params - this ensures backward compatibility
        // and prevents issues from settings corruption
        let default_params = Argon2Params::default();

        let encrypted_data = match fs::read(&self.data_path) {
            Ok(bytes) => EncryptedData::from_bytes(&bytes)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // New vault - create with default params
                let (key, salt) = crypto::generate_test_key_with_params(password_bytes, &default_params)?;
                let test_note_list = NoteList::new();
                let serialized = bincode::serialize(&test_note_list)?;
                let encrypted = crypto::encrypt(&key, &salt, &serialized)?;

                if let Some(state) = CRYPTO_STATE.get() {
                    if let Ok(mut guard) = state.write() {
                        *guard = Some(CryptoState { key, salt });
                    }
                }

                fs::write(&self.data_path, encrypted.to_bytes())?;
                self.note_list = test_note_list;
                return Ok(());
            },
            Err(e) => return Err(e.into()),
        };

        // Always use default params for key derivation - ignore settings.argon2_params
        // This ensures vaults encrypted with default params will always work
        let key = crypto::derive_key_with_params(password_bytes, &encrypted_data.header.salt, &default_params)?;
        
        let decrypted_bytes = crypto::decrypt(&key, &encrypted_data)
            .map_err(|_| anyhow!("Invalid password or corrupted data."))?;

        self.note_list = bincode::deserialize(&decrypted_bytes)?;
        
        if let Some(state) = CRYPTO_STATE.get() {
            if let Ok(mut guard) = state.write() {
                *guard = Some(CryptoState { 
                    key, 
                    salt: encrypted_data.header.salt 
                });
            }
        }

        Ok(())
    }
    
    pub fn change_password(&mut self, old_password: MasterPassword, new_password: MasterPassword) -> Result<()> {
        if !Self::is_unlocked() {
            return Err(anyhow!("Application must be unlocked to change password"));
        }
        
        // Always use default params for consistency
        let default_params = Argon2Params::default();
        
        // Verify old password first
        let encrypted_data = fs::read(&self.data_path)?;
        let encrypted_data = EncryptedData::from_bytes(&encrypted_data)?;
        
        let old_buffer = SecureBuffer::new(old_password.0.clone());
        let old_key = crypto::derive_key_with_params(old_buffer.as_slice(), &encrypted_data.header.salt, &default_params)?;
        
        // Try to decrypt with old password to verify
        crypto::decrypt(&old_key, &encrypted_data)
            .map_err(|_| anyhow!("Current password is incorrect"))?;
        
        // Generate new salt and key with new password (using default params)
        let new_buffer = SecureBuffer::new(new_password.0.clone());
        let (new_key, new_salt) = crypto::generate_test_key_with_params(new_buffer.as_slice(), &default_params)?;
        
        // Re-encrypt with new password
        let serialized = bincode::serialize(&self.note_list)?;
        let encrypted = crypto::encrypt(&new_key, &new_salt, &serialized)?;
        fs::write(&self.data_path, encrypted.to_bytes())?;
        
        // Update crypto state
        if let Some(state) = CRYPTO_STATE.get() {
            if let Ok(mut guard) = state.write() {
                if let Some(ref mut crypto) = *guard {
                    crypto.zeroize();
                }
                *guard = Some(CryptoState { 
                    key: new_key, 
                    salt: new_salt 
                });
            }
        }
        
        Ok(())
    }

    fn save_notes(&self) -> Result<()> {
        let (key, salt) = {
            let state = CRYPTO_STATE.get().ok_or_else(|| anyhow!("Application is locked"))?;
            let guard = state.read().map_err(|_| anyhow!("Lock poisoned"))?;
            let crypto = guard.as_ref().ok_or_else(|| anyhow!("Application is locked"))?;
            (crypto.key.clone(), crypto.salt)
        };

        let serialized = bincode::serialize(&self.note_list)?;
        let encrypted = crypto::encrypt(&key, &salt, &serialized)?;
        fs::write(&self.data_path, encrypted.to_bytes())?;

        Ok(())
    }

    pub fn get_notes(&self) -> Vec<super::data::Note> {
        self.note_list.notes.clone()
    }
    
    pub fn get_folders(&self) -> Vec<String> {
        self.note_list.folders.clone()
    }

    pub fn create_note(&mut self, title: String, content: String) -> Result<u64> {
        let note = super::data::Note::new(title, content);
        let id = note.id;
        self.note_list.add_note(note);
        self.save_notes()?;
        Ok(id)
    }
    
    #[allow(dead_code)]
    pub fn create_note_in_folder(&mut self, title: String, content: String, folder: Option<String>) -> Result<()> {
        let mut note = super::data::Note::new(title, content);
        note.folder = folder;
        self.note_list.add_note(note);
        self.save_notes()
    }

    pub fn update_note(&mut self, id: u64, title: String, content: String, folder: Option<String>) -> Result<()> {
        if self.note_list.update_note(id, title, content, folder) {
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
    
    pub fn toggle_pin(&mut self, id: u64) -> Result<bool> {
        if self.note_list.toggle_pin(id) {
            self.save_notes()?;
            // Return the new pin state
            let is_pinned = self.note_list.notes.iter()
                .find(|n| n.id == id)
                .map(|n| n.pinned)
                .unwrap_or(false);
            Ok(is_pinned)
        } else {
            Err(anyhow!("Note with ID {} not found", id))
        }
    }
    
    pub fn add_folder(&mut self, name: String) -> Result<()> {
        self.note_list.add_folder(name);
        self.save_notes()
    }
    
    pub fn delete_folder(&mut self, name: &str) -> Result<()> {
        self.note_list.delete_folder(name);
        self.save_notes()
    }

    pub fn export_all_encrypted(&self, export_path: &PathBuf) -> Result<()> {
        let (key, salt) = {
            let state = CRYPTO_STATE.get().ok_or_else(|| anyhow!("Application is locked"))?;
            let guard = state.read().map_err(|_| anyhow!("Lock poisoned"))?;
            let crypto = guard.as_ref().ok_or_else(|| anyhow!("Application is locked"))?;
            (crypto.key.clone(), crypto.salt)
        };

        let serialized = bincode::serialize(&self.note_list)?;
        let encrypted = crypto::encrypt(&key, &salt, &serialized)?;
        fs::write(export_path, encrypted.to_bytes())?;

        Ok(())
    }

    pub fn import_encrypted(&mut self, import_path: &PathBuf, master_password: MasterPassword) -> Result<()> {
        let password_buffer = SecureBuffer::new(master_password.0.clone());
        
        let encrypted_data = fs::read(import_path).map_err(|e| anyhow!("Failed to read import file: {}", e))?;
        let encrypted_data = EncryptedData::from_bytes(&encrypted_data)?;

        // Try with default params for imported files
        let key = crypto::derive_key(password_buffer.as_slice(), &encrypted_data.header.salt)?;
        let decrypted_bytes = crypto::decrypt(&key, &encrypted_data)?;
        let imported_note_list: NoteList = bincode::deserialize(&decrypted_bytes)?;

        // Import folders
        for folder in &imported_note_list.folders {
            self.note_list.add_folder(folder.clone());
        }

        for mut note in imported_note_list.notes.clone() {
            // If the imported note's ID already exists in the vault, assign a new
            // unique ID so the existing note is not silently shadowed or clobbered.
            if self.note_list.notes.iter().any(|n| n.id == note.id) {
                let seq = super::data::ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed) & 0xF_FFFF;
                note.id = (chrono::Utc::now().timestamp_millis() as u64) << 20 | seq;
            }
            self.note_list.add_note(note);
        }
        
        self.save_notes()?;
        Ok(())
    }
}

impl Drop for CoreManager {
    fn drop(&mut self) {
        // Zeroize all sensitive data
        self.note_list.zeroize();
    }
}
