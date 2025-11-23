mod core;

use gtk4 as gtk;
use gtk::{
    prelude::*,
    glib,
    Application, ApplicationWindow, Label, ListBoxRow,
};
use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};

use core::manager::CoreManager;

// Global application state
static CORE_MANAGER: OnceCell<Arc<Mutex<CoreManager>>> = OnceCell::new();
static TOKIO_RUNTIME: OnceCell<tokio::runtime::Runtime> = OnceCell::new();

const APP_ID: &str = "com.jegly.NocturneNotes";

fn main() -> glib::ExitCode {
    // Initialize tokio runtime
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    if TOKIO_RUNTIME.set(runtime).is_err() {
        eprintln!("Failed to set tokio runtime");
        return glib::ExitCode::FAILURE;
    }

    match CoreManager::new() {
        Ok(manager) => {
            if CORE_MANAGER.set(Arc::new(Mutex::new(manager))).is_err() {
                eprintln!("Failed to set CoreManager");
                return glib::ExitCode::FAILURE;
            }
        },
        Err(e) => {
            eprintln!("Failed to initialize CoreManager: {}", e);
            return glib::ExitCode::FAILURE;
        }
    }

    let application = Application::builder()
        .application_id(APP_ID)
        .build();

    application.connect_startup(|_| {
        load_css();
    });

    application.connect_activate(build_ui);

    application.run()
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    let css = r#"
        /* Dracula Theme Colors */
        @define-color background #282a36;
        @define-color current-line #44475a;
        @define-color foreground #f8f8f2;
        @define-color green #50fa7b;
        @define-color comment #6272a4;
        @define-color red #ff5555;
        @define-color selection #44475a;

        * {
            background-color: @background;
            color: @foreground;
            border-color: @comment;
        }
        
        /* Text selection highlighting */
        selection {
            background-color: @selection;
            color: @foreground;
        }
        
        .title-label { font-size: 1.1em; font-weight: bold; color: @green; }
        .note-list-row { padding: 5px; border-bottom: 1px solid @comment; }
        .note-list-row:selected { background-color: @current-line; }
        GtkTextView { background-color: @background; color: @foreground; border: 1px solid @comment; padding: 10px; }
        GtkTextView selection { background-color: @selection; }
        GtkEntry { background-color: @current-line; color: @foreground; border: 1px solid @comment; padding: 5px; }
        GtkButton { background-color: @current-line; color: @green; border: 1px solid @green; padding: 5px 10px; }
        GtkButton:hover { background-color: @comment; }
        .destructive-action { color: @red; border-color: @red; }
    "#;
    provider.load_from_data(css);
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_ui(app: &Application) {
    if !CoreManager::is_unlocked() {
        show_password_screen(app);
    } else {
        show_main_window(app);
    }
}

fn show_password_screen(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Nocturne Notes - Unlock")
        .default_width(300)
        .default_height(150)
        .modal(true)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 10);
    vbox.set_margin_top(20);
    vbox.set_margin_bottom(20);
    vbox.set_margin_start(20);
    vbox.set_margin_end(20);

    let label = Label::new(Some("Enter Master Password"));
    label.add_css_class("title-label");

    let password_entry = gtk::PasswordEntry::new();
    password_entry.set_placeholder_text(Some("Password"));

    let status_label = Arc::new(Label::new(None));
    status_label.set_halign(gtk::Align::Start);
    status_label.set_markup("<span foreground='#6272a4'>Enter password to unlock or create new file.</span>");

    let unlock_button = gtk::Button::with_label("Unlock");
    unlock_button.add_css_class("suggested-action");

    let window_clone = window.clone();
    let app_clone = app.clone();
    let status_label_clone = status_label.clone();
    let password_entry_clone = password_entry.clone();

    unlock_button.connect_clicked(move |_| {
        let password = password_entry_clone.text().to_string();
        if password.is_empty() {
            status_label_clone.set_markup("<span foreground='#ff5555'>Password cannot be empty.</span>");
            return;
        }

        let manager_rc = CORE_MANAGER.get().unwrap().clone();
        let master_password = core::data::MasterPassword::from(password.as_str());

        let result = manager_rc.lock().unwrap().unlock(master_password);
        match result {
            Ok(_) => {
                window_clone.close();
                show_main_window(&app_clone);
            },
            Err(e) => {
                status_label_clone.set_markup(&format!("<span foreground='#ff5555'>Unlock failed: {}</span>", e));
            }
        };
    });

    vbox.append(&label);
    vbox.append(&password_entry);
    vbox.append(status_label.as_ref());
    vbox.append(&unlock_button);

    window.set_child(Some(&vbox));
    window.present();
}

fn show_main_window(app: &Application) {
    let manager_rc = CORE_MANAGER.get().unwrap().clone();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Nocturne Notes")
        .default_width(800)
        .default_height(600)
        .build();

    let main_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 5);
    sidebar.set_width_request(250);
    sidebar.set_margin_top(10);
    sidebar.set_margin_bottom(10);
    sidebar.set_margin_start(10);
    sidebar.set_margin_end(10);

    let new_note_button = gtk::Button::with_label("New Note");
    new_note_button.add_css_class("suggested-action");

    let note_list_box = Arc::new(gtk::ListBox::new());
    note_list_box.set_selection_mode(gtk::SelectionMode::Single);

    let scrolled_window = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(note_list_box.as_ref())
        .vexpand(true)
        .build();

    let export_button = gtk::Button::with_label("Export Notes");
    let import_button = gtk::Button::with_label("Import Notes");

    sidebar.append(&new_note_button);
    sidebar.append(&scrolled_window);
    sidebar.append(&export_button);
    sidebar.append(&import_button);

    let editor_area = gtk::Box::new(gtk::Orientation::Vertical, 10);
    editor_area.set_margin_top(10);
    editor_area.set_margin_bottom(10);
    editor_area.set_margin_start(10);
    editor_area.set_margin_end(10);
    editor_area.set_hexpand(true);

    let title_entry = Arc::new(gtk::Entry::new());
    title_entry.set_placeholder_text(Some("Note Title"));

    let content_buffer = Arc::new(gtk::TextBuffer::new(None));
    let content_view = gtk::TextView::builder()
        .buffer(content_buffer.as_ref())
        .vexpand(true)
        .editable(true)
        .wrap_mode(gtk::WrapMode::Word)
        .build();

    let editor_scrolled_window = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&content_view)
        .vexpand(true)
        .build();

    let status_label = Arc::new(Label::new(Some("Ready.")));
    status_label.set_halign(gtk::Align::Start);

    let save_button = Arc::new(gtk::Button::with_label("Save Note"));
    save_button.add_css_class("suggested-action");
    save_button.set_sensitive(false);

    let delete_button = Arc::new(gtk::Button::with_label("Delete Note"));
    delete_button.add_css_class("destructive-action");
    delete_button.set_sensitive(false);

    let status_bar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    status_bar.append(status_label.as_ref());
    status_bar.append(save_button.as_ref());
    status_bar.append(delete_button.as_ref());

    editor_area.append(title_entry.as_ref());
    editor_area.append(&editor_scrolled_window);
    editor_area.append(&status_bar);

    main_box.append(&sidebar);
    main_box.append(&editor_area);

    window.set_child(Some(&main_box));

    // State tracking
    let active_note_id = Arc::new(Mutex::new(None::<u64>));
    let row_ids: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));

    // Update list UI function
    let update_list_ui = {
        let note_list_box = note_list_box.clone();
        let manager_rc = manager_rc.clone();
        let row_ids = row_ids.clone();

        move || {
            println!("Refreshing note list...");
            while let Some(child) = note_list_box.first_child() {
                note_list_box.remove(&child);
            }
            row_ids.lock().unwrap().clear();

            let notes = manager_rc.lock().unwrap().get_notes();
            for note in notes {
                let row = ListBoxRow::new();
                row.add_css_class("note-list-row");

                let label = Label::new(Some(&note.title));
                label.set_halign(gtk::Align::Start);
                label.add_css_class("title-label");

                row.set_child(Some(&label));
                note_list_box.append(&row);

                row_ids.lock().unwrap().push(note.id);
            }

            note_list_box.show();
        }
    };

    // Initial list load
    update_list_ui();

    // Row selection handler
    note_list_box.connect_row_selected(glib::clone!(@strong manager_rc,
        @strong title_entry, @strong content_buffer, @strong delete_button,
        @strong save_button, @strong active_note_id, @strong status_label,
        @strong row_ids => move |_, row_opt| {
            if let Some(row) = row_opt {
                let idx = row.index();
                if idx >= 0 {
                    let idx_usize = idx as usize;
                    let id_opt = row_ids.lock().unwrap().get(idx_usize).copied();
                    if let Some(id) = id_opt {
                        println!("Row selected id={}", id);
                        let note_opt = {
                            let mgr = manager_rc.lock().unwrap();
                            mgr.get_notes().into_iter().find(|n| n.id == id)
                        };

                        if let Some(note) = note_opt {
                            title_entry.set_text(&note.title);
                            content_buffer.set_text(&note.content);
                            *active_note_id.lock().unwrap() = Some(id);
                            delete_button.set_sensitive(true);
                            save_button.set_sensitive(true);
                            status_label.set_text(&format!("Note loaded: {}", note.title));
                        } else {
                            status_label.set_text("Note not found.");
                            delete_button.set_sensitive(false);
                            save_button.set_sensitive(false);
                            *active_note_id.lock().unwrap() = None;
                        }
                    }
                }
            } else {
                delete_button.set_sensitive(false);
                save_button.set_sensitive(false);
                *active_note_id.lock().unwrap() = None;
            }
    }));

    // New Note Button
    new_note_button.connect_clicked(glib::clone!(@strong manager_rc,
        @strong title_entry, @strong content_buffer,
        @strong active_note_id, @strong delete_button, @strong save_button,
        @strong note_list_box, @strong status_label, @strong row_ids => move |_| {

        println!("New Note clicked");

        // Clear the editor first
        title_entry.set_text("");
        content_buffer.set_text("");
        *active_note_id.lock().unwrap() = None;
        delete_button.set_sensitive(false);
        save_button.set_sensitive(false);

        let manager_clone = manager_rc.clone();
        let status_clone = status_label.clone();
        let list_box_clone = note_list_box.clone();
        let row_ids_clone = row_ids.clone();

        status_label.set_text("Creating note...");

        // Create async channel for communication
        let (sender, receiver) = async_channel::unbounded();

        // Spawn tokio task
        let runtime = TOKIO_RUNTIME.get().unwrap();
        glib::spawn_future_local(async move {
            let _guard = runtime.enter();
            let result = tokio::task::spawn_blocking(move || {
                let mut manager = manager_clone.lock().unwrap();
                // Create empty note with "Untitled"
                let create_result = manager.create_note("Untitled".to_string(), String::new());
                // Get updated notes list while we have the lock
                let notes = manager.get_notes();
                (create_result, notes)
            }).await;

            // Send result through async channel
            let _ = sender.send(result).await;
        });

        // Receive in GTK main thread
        glib::spawn_future_local(async move {
            if let Ok(result) = receiver.recv().await {
            match result {
                Ok((Ok(_), notes)) => {
                    status_clone.set_text("New note created.");
                    
                    // Update UI with provided notes list (no locking!)
                    while let Some(child) = list_box_clone.first_child() {
                        list_box_clone.remove(&child);
                    }
                    row_ids_clone.lock().unwrap().clear();

                    for note in notes {
                        let row = ListBoxRow::new();
                        row.add_css_class("note-list-row");

                        let label = Label::new(Some(&note.title));
                        label.set_halign(gtk::Align::Start);
                        label.add_css_class("title-label");

                        row.set_child(Some(&label));
                        list_box_clone.append(&row);

                        row_ids_clone.lock().unwrap().push(note.id);
                    }
                    
                    // Select the first note (newest) and load it
                    if let Some(row) = list_box_clone.row_at_index(0) {
                        list_box_clone.select_row(Some(&row));
                        // The row selection handler will load the note into the editor
                    }
                },
                Ok((Err(e), _)) => {
                    status_clone.set_text(&format!("Error creating: {}", e));
                },
                Err(e) => {
                    status_clone.set_text(&format!("Error: {}", e));
                }
            }
            } // Close the if let Ok
        }); // Close the spawn_future_local async block
    }));

    // Save Note Button
    save_button.connect_clicked(glib::clone!(@strong manager_rc,
        @strong active_note_id, @strong title_entry, @strong content_buffer,
        @strong status_label, @strong note_list_box,
        @strong row_ids => move |_| {

        println!("Save Note clicked");
        if let Some(id) = *active_note_id.lock().unwrap() {
            let title = title_entry.text().to_string();
            let content = content_buffer.text(
                &content_buffer.start_iter(),
                &content_buffer.end_iter(),
                false,
            ).to_string();

            let manager_clone = manager_rc.clone();
            let status_clone = status_label.clone();
            let list_box_clone = note_list_box.clone();
            let row_ids_clone = row_ids.clone();

            status_label.set_text("Saving note...");

            // Create async channel for communication
            let (sender, receiver) = async_channel::unbounded();

            // Spawn tokio task
            let runtime = TOKIO_RUNTIME.get().unwrap();
            glib::spawn_future_local(async move {
                let _guard = runtime.enter();
                let result = tokio::task::spawn_blocking(move || {
                    let mut manager = manager_clone.lock().unwrap();
                    let save_result = manager.update_note(id, title.clone(), content.clone());
                    // Get updated notes list while we have the lock
                    let notes = manager.get_notes();
                    (save_result, notes, id)
                }).await;

                // Send result through async channel
                let _ = sender.send(result).await;
            });

            // Receive in GTK main thread
            glib::spawn_future_local(async move {
                if let Ok(result) = receiver.recv().await {
                match result {
                    Ok((Ok(_), notes, saved_id)) => {
                        status_clone.set_text("Note saved.");
                        
                        // Find the row for this note and update just its title
                        let ids = row_ids_clone.lock().unwrap();
                        for i in 0..ids.len() {
                            if ids[i] == saved_id {
                                // Find the updated note
                                if let Some(updated_note) = notes.iter().find(|n| n.id == saved_id) {
                                    // Update the row's label
                                    if let Some(row) = list_box_clone.row_at_index(i as i32) {
                                        if let Some(label) = row.child().and_then(|w| w.downcast::<Label>().ok()) {
                                            label.set_text(&updated_note.title);
                                        }
                                    }
                                }
                                break;
                            }
                        }
                    },
                    Ok((Err(e), _, _)) => {
                        status_clone.set_text(&format!("Error saving: {}", e));
                    },
                    Err(e) => {
                        status_clone.set_text(&format!("Error: {}", e));
                    }
                }
                } // Close the if let Ok
            });
        } else {
            status_label.set_text("No note selected to save.");
        }
    }));

    // Delete Note Button
    delete_button.connect_clicked(glib::clone!(@strong manager_rc,
        @strong active_note_id,
        @strong title_entry, @strong content_buffer,
        @strong delete_button, @strong save_button, @strong status_label,
        @strong note_list_box, @strong row_ids => move |_| {

        println!("Delete Note clicked");
        if let Some(id) = *active_note_id.lock().unwrap() {
            let manager_clone = manager_rc.clone();
            let status_clone = status_label.clone();
            let title_clone = title_entry.clone();
            let content_clone = content_buffer.clone();
            let active_clone = active_note_id.clone();
            let delete_clone = delete_button.clone();
            let save_clone = save_button.clone();
            let list_clone = note_list_box.clone();
            let row_ids_clone = row_ids.clone();

            status_label.set_text("Deleting note...");
            delete_button.set_sensitive(false);

            // Create async channel for communication
            let (sender, receiver) = async_channel::unbounded();

            // Spawn tokio task
            let runtime = TOKIO_RUNTIME.get().unwrap();
            glib::spawn_future_local(async move {
                let _guard = runtime.enter();
                let result = tokio::task::spawn_blocking(move || {
                    let mut manager = manager_clone.lock().unwrap();
                    let delete_result = manager.delete_note(id);
                    // Get updated notes list while we have the lock
                    let notes = manager.get_notes();
                    (delete_result, notes)
                }).await;

                // Send result through async channel
                let _ = sender.send(result).await;
            });

            // Receive in GTK main thread
            glib::spawn_future_local(async move {
                if let Ok(result) = receiver.recv().await {
                match result {
                    Ok((Ok(_), notes)) => {
                        status_clone.set_text("Note deleted.");
                        title_clone.set_text("");
                        content_clone.set_text("");
                        *active_clone.lock().unwrap() = None;
                        delete_clone.set_sensitive(false);
                        save_clone.set_sensitive(false);
                        
                        // Update UI with provided notes list (no locking!)
                        while let Some(child) = list_clone.first_child() {
                            list_clone.remove(&child);
                        }
                        row_ids_clone.lock().unwrap().clear();

                        for note in notes {
                            let row = ListBoxRow::new();
                            row.add_css_class("note-list-row");

                            let label = Label::new(Some(&note.title));
                            label.set_halign(gtk::Align::Start);
                            label.add_css_class("title-label");

                            row.set_child(Some(&label));
                            list_clone.append(&row);

                            row_ids_clone.lock().unwrap().push(note.id);
                        }

                        if let Some(row) = list_clone.selected_row() {
                            list_clone.unselect_row(&row);
                        }
                    },
                    Ok((Err(e), _)) => {
                        status_clone.set_text(&format!("Error deleting: {}", e));
                        delete_clone.set_sensitive(true);
                    },
                    Err(e) => {
                        status_clone.set_text(&format!("Error: {}", e));
                        delete_clone.set_sensitive(true);
                    }
                }
                } // Close the if let Ok
            });
        } else {
            status_label.set_text("No note selected to delete.");
        }
    }));

    // Export Button
    export_button.connect_clicked(glib::clone!(@strong manager_rc, @strong status_label, @strong window => move |_| {
        println!("Export clicked");
        
        let file_chooser = gtk::FileChooserDialog::new(
            Some("Export Notes"),
            Some(&window),
            gtk::FileChooserAction::Save,
            &[("Cancel", gtk::ResponseType::Cancel), ("Export", gtk::ResponseType::Accept)],
        );
        
        file_chooser.set_current_name("notes_export.dat");
        
        // Add file filter for .dat files
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Nocturne Notes Files (*.dat)"));
        filter.add_pattern("*.dat");
        file_chooser.add_filter(&filter);
        
        // Add "All Files" filter as fallback
        let filter_all = gtk::FileFilter::new();
        filter_all.set_name(Some("All Files"));
        filter_all.add_pattern("*");
        file_chooser.add_filter(&filter_all);
        
        let manager_clone = manager_rc.clone();
        let status_clone = status_label.clone();
        
        file_chooser.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        let manager_clone2 = manager_clone.clone();
                        let status_clone2 = status_clone.clone();
                        let path_clone = path.clone();
                        
                        status_clone.set_text("Exporting notes...");
                        
                        // Create async channel for communication
                        let (sender, receiver) = async_channel::unbounded();
                        
                        // Spawn tokio task
                        let runtime = TOKIO_RUNTIME.get().unwrap();
                        glib::spawn_future_local(async move {
                            let _guard = runtime.enter();
                            let result = tokio::task::spawn_blocking(move || {
                                let manager = manager_clone2.lock().unwrap();
                                manager.export_all_encrypted(&path_clone)
                            }).await;
                            
                            // Send result through async channel
                            let _ = sender.send((result, path)).await;
                        });
                        
                        // Receive in GTK main thread
                        glib::spawn_future_local(async move {
                            if let Ok((result, path)) = receiver.recv().await {
                            match result {
                                Ok(Ok(_)) => {
                                    status_clone2.set_text(&format!("Notes exported to: {}", path.display()));
                                },
                                Ok(Err(e)) => {
                                    status_clone2.set_text(&format!("Export failed: {}", e));
                                },
                                Err(e) => {
                                    status_clone2.set_text(&format!("Error: {}", e));
                                }
                            }
                            } // Close the if let Ok
                        });
                    }
                }
            }
            dialog.close();
        });
        
        file_chooser.show();
    }));

    // Import Button
    import_button.connect_clicked(glib::clone!(@strong manager_rc, @strong status_label, @strong window, @strong note_list_box, @strong row_ids => move |_| {
        println!("Import clicked");
        
        let file_chooser = gtk::FileChooserDialog::new(
            Some("Import Notes"),
            Some(&window),
            gtk::FileChooserAction::Open,
            &[("Cancel", gtk::ResponseType::Cancel), ("Import", gtk::ResponseType::Accept)],
        );
        
        // Add file filter for .dat files
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Nocturne Notes Files (*.dat)"));
        filter.add_pattern("*.dat");
        file_chooser.add_filter(&filter);
        
        // Add "All Files" filter as fallback
        let filter_all = gtk::FileFilter::new();
        filter_all.set_name(Some("All Files"));
        filter_all.add_pattern("*");
        file_chooser.add_filter(&filter_all);
        
        let manager_clone = manager_rc.clone();
        let status_clone = status_label.clone();
        let window_clone = window.clone();
        let list_box_clone = note_list_box.clone();
        let row_ids_clone = row_ids.clone();
        
        file_chooser.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        // Show password dialog for import
                        show_import_password_dialog(
                            &window_clone, 
                            manager_clone.clone(), 
                            status_clone.clone(),
                            list_box_clone.clone(),
                            row_ids_clone.clone(),
                            path
                        );
                    }
                }
            }
            dialog.close();
        });
        
        file_chooser.show();
    }));

    window.present();
}

fn show_import_password_dialog(
    parent: &ApplicationWindow,
    manager_rc: Arc<Mutex<CoreManager>>,
    status_label: Arc<Label>,
    note_list_box: Arc<gtk::ListBox>,
    row_ids: Arc<Mutex<Vec<u64>>>,
    import_path: std::path::PathBuf,
) {
    let dialog = gtk::Window::builder()
        .title("Import Password")
        .modal(true)
        .transient_for(parent)
        .default_width(300)
        .default_height(150)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 10);
    vbox.set_margin_top(20);
    vbox.set_margin_bottom(20);
    vbox.set_margin_start(20);
    vbox.set_margin_end(20);

    let label = Label::new(Some("Enter password for import file:"));
    label.add_css_class("title-label");

    let password_entry = gtk::PasswordEntry::new();
    password_entry.set_placeholder_text(Some("Password"));

    let import_status_label = Arc::new(Label::new(None));
    import_status_label.set_halign(gtk::Align::Start);

    let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    button_box.set_halign(gtk::Align::End);

    let cancel_button = gtk::Button::with_label("Cancel");
    let import_button = gtk::Button::with_label("Import");
    import_button.add_css_class("suggested-action");

    button_box.append(&cancel_button);
    button_box.append(&import_button);

    vbox.append(&label);
    vbox.append(&password_entry);
    vbox.append(import_status_label.as_ref());
    vbox.append(&button_box);

    dialog.set_child(Some(&vbox));

    let dialog_clone = dialog.clone();
    cancel_button.connect_clicked(move |_| {
        dialog_clone.close();
    });

    let dialog_clone2 = dialog.clone();
    let import_status_clone = import_status_label.clone();
    import_button.connect_clicked(move |_| {
        let password = password_entry.text().to_string();
        if password.is_empty() {
            import_status_clone.set_markup("<span foreground='#ff5555'>Password cannot be empty.</span>");
            return;
        }

        let manager_clone = manager_rc.clone();
        let status_clone = status_label.clone();
        let path_clone = import_path.clone();
        let dialog_to_close = dialog_clone2.clone();
        let import_status_clone2 = import_status_clone.clone();
        let list_box_clone = note_list_box.clone();
        let row_ids_clone = row_ids.clone();
        
        import_status_clone.set_text("Importing...");
        
        // Create async channel for communication
        let (sender, receiver) = async_channel::unbounded();
        
        // Spawn tokio task
        let runtime = TOKIO_RUNTIME.get().unwrap();
        glib::spawn_future_local(async move {
            let _guard = runtime.enter();
            let result = tokio::task::spawn_blocking(move || {
                let mut manager = manager_clone.lock().unwrap();
                let master_password = core::data::MasterPassword::from(password.as_str());
                let import_result = manager.import_encrypted(&path_clone, master_password);
                // Get updated notes list while we have the lock
                let notes = manager.get_notes();
                (import_result, notes)
            }).await;
            
            // Send result through async channel
            let _ = sender.send(result).await;
        });
        
        // Receive in GTK main thread
        glib::spawn_future_local(async move {
            if let Ok(result) = receiver.recv().await {
            match result {
                Ok((Ok(_), notes)) => {
                    status_clone.set_text("Notes imported successfully.");
                    
                    // Update UI with provided notes list (no locking!)
                    while let Some(child) = list_box_clone.first_child() {
                        list_box_clone.remove(&child);
                    }
                    row_ids_clone.lock().unwrap().clear();

                    for note in notes {
                        let row = ListBoxRow::new();
                        row.add_css_class("note-list-row");

                        let label = Label::new(Some(&note.title));
                        label.set_halign(gtk::Align::Start);
                        label.add_css_class("title-label");

                        row.set_child(Some(&label));
                        list_box_clone.append(&row);

                        row_ids_clone.lock().unwrap().push(note.id);
                    }
                    
                    dialog_to_close.close();
                },
                Ok((Err(e), _)) => {
                    import_status_clone2.set_markup(&format!("<span foreground='#ff5555'>Import failed: {}</span>", e));
                },
                Err(e) => {
                    import_status_clone2.set_markup(&format!("<span foreground='#ff5555'>Error: {}</span>", e));
                }
            }
            } // Close the if let Ok
        });
    });

    dialog.present();
}
