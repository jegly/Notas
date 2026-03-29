mod core;

use gtk4 as gtk;
use gtk::{
    prelude::*,
    glib,
    Application, ApplicationWindow, Label, ListBoxRow,
};
use libadwaita as adw;
use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use core::manager::CoreManager;
use core::data::{AppSettings, AppTheme, EditorFont};

static CORE_MANAGER: OnceCell<Arc<Mutex<CoreManager>>> = OnceCell::new();
static TOKIO_RUNTIME: OnceCell<tokio::runtime::Runtime> = OnceCell::new();

const APP_ID: &str = "com.jegly.Notas";

thread_local! {
    static LAST_ACTIVITY: RefCell<Instant> = RefCell::new(Instant::now());
    static CLIPBOARD_TIMER: RefCell<Option<glib::SourceId>> = RefCell::new(None);
    static CURRENT_THEME: RefCell<AppTheme> = RefCell::new(AppTheme::Dark);
    static CSS_PROVIDER: RefCell<Option<gtk::CssProvider>> = RefCell::new(None);
    static EDITOR_FONT: RefCell<EditorFont> = RefCell::new(EditorFont::default());
    static EDITOR_FONT_SIZE: RefCell<u32> = RefCell::new(12);
    static SHOW_NOTE_TITLE: RefCell<bool> = RefCell::new(true);
}

fn main() -> glib::ExitCode {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    if TOKIO_RUNTIME.set(runtime).is_err() {
        eprintln!("Failed to set tokio runtime");
        return glib::ExitCode::FAILURE;
    }

    match CoreManager::new() {
        Ok(manager) => {
            let settings = manager.get_settings();
            CURRENT_THEME.with(|t| *t.borrow_mut() = settings.theme.clone());
            EDITOR_FONT.with(|f| *f.borrow_mut() = settings.editor_font.clone());
            EDITOR_FONT_SIZE.with(|s| *s.borrow_mut() = settings.editor_font_size);
            SHOW_NOTE_TITLE.with(|s| *s.borrow_mut() = settings.show_note_title);
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
        adw::init().expect("Failed to initialize libadwaita");
        let theme = CURRENT_THEME.with(|t| t.borrow().clone());
        load_css(&theme);
    });

    application.connect_activate(build_ui);

    application.run()
}

fn get_editor_font_css() -> String {
    let font_family = EDITOR_FONT.with(|f| f.borrow().to_css_family().to_string());
    let font_size = EDITOR_FONT_SIZE.with(|s| *s.borrow());
    format!(
        ".content-view {{ font-family: {}; font-size: {}pt; }}
         .content-view text {{ font-family: {}; font-size: {}pt; }}",
        font_family, font_size, font_family, font_size
    )
}

fn get_dark_css() -> String {
    let editor_css = get_editor_font_css();
    format!(r#"
        /* Dark Theme - Smooth gradients, subtle styling */
        @define-color bg_color #080808;
        @define-color bg_mid #101014;
        @define-color bg_light #18181c;
        @define-color surface_color #0e0e12;
        @define-color overlay_color #1a1a1f;
        @define-color text_color #d8d8d8;
        @define-color subtext_color #707070;
        @define-color accent_gray #505050;
        @define-color accent_light #888888;
        @define-color border_color #252528;
        @define-color focus_color #404045;

        /* ========== AGGRESSIVE FOCUS REMOVAL ========== */
        *, *:focus, *:focus-within, *:focus-visible {{
            outline: none;
            outline-width: 0;
            outline-style: none;
            box-shadow: none;
            -gtk-outline-radius: 0;
        }}
        
        entry, entry:focus, entry:focus-within,
        textview, textview:focus, textview:focus-within,
        text, text:focus,
        password-entry, password-entry:focus {{
            outline: none;
            outline-width: 0;
            box-shadow: none;
            -gtk-outline-radius: 0;
        }}
        
        .password-entry, .password-entry:focus, .password-entry:focus-within,
        .title-entry, .title-entry:focus, .title-entry:focus-within,
        .search-entry, .search-entry:focus, .search-entry:focus-within {{
            outline: none;
            outline-width: 0;
            box-shadow: none;
            border-color: @focus_color;
        }}

        /* Smooth gradient background */
        window, .background {{ 
            background: linear-gradient(160deg, 
                #18181c 0%, 
                #101014 25%, 
                #0c0c0f 50%, 
                #080808 75%,
                #050506 100%
            );
            color: @text_color; 
        }}
        
        /* ========== CUSTOM HEADER BAR ========== */
        .custom-headerbar {{
            background: linear-gradient(180deg, #1a1a1e 0%, #101014 100%);
            border-bottom: 1px solid @border_color;
            padding: 4px 8px;
            min-height: 32px;
        }}
        
        .headerbar-title {{
            font-family: 'DotGothic16', 'Noto Sans', monospace;
            font-size: 0.95em;
            font-weight: 600;
            color: @text_color;
        }}
        
        /* Traffic light buttons - perfect circles */
        .traffic-btn {{
            min-width: 12px;
            min-height: 12px;
            max-width: 12px;
            max-height: 12px;
            padding: 0;
            margin: 0 4px;
            border-radius: 6px;
            border: none;
            font-size: 0;
            background: @accent_gray;
            -gtk-icon-size: 0;
        }}
        
        .traffic-btn:hover {{
            opacity: 0.8;
        }}
        
        .traffic-close {{
            background-color: #ff5f57;
            background-image: none;
        }}
        .traffic-close:hover {{
            background-color: #ff3b30;
            background-image: none;
        }}
        
        .traffic-minimize {{
            background-color: #ffbd2e;
            background-image: none;
        }}
        .traffic-minimize:hover {{
            background-color: #ff9500;
            background-image: none;
        }}
        
        .traffic-maximize {{
            background-color: #28c840;
            background-image: none;
        }}
        .traffic-maximize:hover {{
            background-color: #00b341;
            background-image: none;
        }}
        
        /* Title toggle switch */
        .title-toggle {{
            min-width: 36px;
            min-height: 18px;
            border-radius: 9px;
            background-color: @surface_color;
            border: 1px solid @border_color;
        }}
        .title-toggle:checked {{
            background-color: @accent_gray;
        }}
        .title-toggle slider {{
            min-width: 14px;
            min-height: 14px;
            border-radius: 7px;
            background-color: @subtext_color;
        }}
        .title-toggle:checked slider {{
            background-color: @text_color;
        }}
        
        /* Compact sidebar */
        .sidebar {{ 
            background: linear-gradient(180deg, 
                #141416 0%, 
                #101012 30%,
                #0c0c0e 60%,
                #08080a 100%
            );
            border-right: 1px solid @border_color; 
        }}
        
        .sidebar-header {{ 
            padding: 10px 10px; 
            border-bottom: 1px solid @border_color;
            background: transparent;
        }}
        
        .app-title {{ 
            font-family: 'DotGothic16', 'Noto Sans', monospace;
            font-size: 1.3em; 
            font-weight: bold;
            color: #e0e0e0;
        }}
        
        .lock-title {{ 
            font-family: 'DotGothic16', 'Noto Sans', monospace;
            font-size: 3.2em; 
            font-weight: bold;
            color: #f0f0f0;
            margin-bottom: 12px; 
        }}
        
        .lock-screen {{ 
            background: linear-gradient(160deg, 
                #1a1a1e 0%, 
                #121215 20%,
                #0a0a0c 45%,
                #060608 70%,
                #040405 100%
            );
        }}
        
        .search-entry {{ 
            background-color: @surface_color; 
            border: 1px solid @border_color; 
            border-radius: 5px; 
            padding: 6px 8px; 
            margin: 6px 8px; 
            color: @text_color; 
            outline: none;
        }}
        .search-entry:focus {{ 
            border-color: @focus_color; 
            outline: none;
            box-shadow: none;
        }}
        
        .note-list {{ background-color: transparent; }}
        .note-list row {{ 
            padding: 8px 10px; 
            margin: 1px 4px; 
            border-radius: 5px; 
            background-color: transparent; 
            border: 1px solid transparent;
        }}
        .note-list row:hover {{ 
            background-color: alpha(@overlay_color, 0.6);
            border-color: @border_color;
        }}
        .note-list row:selected {{ 
            background-color: @overlay_color;
            border-color: @accent_gray;
        }}
        
        .note-title {{ font-weight: 600; font-size: 0.9em; color: @text_color; }}
        .note-preview {{ font-size: 0.78em; color: @subtext_color; margin-top: 2px; }}
        .note-date {{ font-size: 0.7em; color: alpha(@subtext_color, 0.6); margin-top: 2px; }}
        .note-pinned {{ color: #a08050; }}
        
        .editor-area {{ 
            background: linear-gradient(160deg, 
                #18181c 0%, 
                #101014 25%, 
                #0c0c0f 50%, 
                #080808 75%,
                #050506 100%
            );
            padding: 16px; 
        }}
        
        .title-entry {{ 
            font-size: 1.3em; 
            font-weight: bold; 
            background-color: transparent; 
            border: none; 
            border-bottom: 1px solid @border_color; 
            border-radius: 0; 
            padding: 6px 4px; 
            margin-bottom: 12px; 
            color: @text_color;
            outline: none;
        }}
        .title-entry:focus {{ 
            border-bottom-color: @focus_color;
            outline: none;
            box-shadow: none;
        }}
        
        .content-view {{ 
            background-color: @surface_color; 
            border-radius: 6px; 
            padding: 12px; 
            color: @text_color;
            border: 1px solid @border_color;
            outline: none;
        }}
        .content-view text {{ background-color: transparent; color: @text_color; }}
        .content-view text selection {{ background-color: alpha(@accent_gray, 0.4); color: @text_color; }}
        .content-view:focus {{ 
            border-color: @focus_color;
            outline: none;
            box-shadow: none;
        }}
        
        {}
        
        .status-bar {{ 
            background: linear-gradient(180deg, @surface_color 0%, #060608 100%);
            padding: 6px 10px; 
            border-top: 1px solid @border_color; 
        }}
        .status-text {{ color: @subtext_color; font-size: 0.8em; }}
        
        .action-button {{ 
            background: linear-gradient(180deg, #1c1c20 0%, #101014 100%);
            color: @text_color; 
            border: 1px solid @accent_gray; 
            border-radius: 5px; 
            padding: 7px 14px; 
            font-weight: 600;
            font-size: 0.9em;
            outline: none;
        }}
        .action-button:hover {{ 
            background: linear-gradient(180deg, #222226 0%, #161618 100%);
            border-color: @accent_light;
        }}
        .action-button:focus {{
            outline: none;
            box-shadow: none;
        }}
        
        .secondary-button {{ 
            background: linear-gradient(180deg, #161618 0%, #0e0e10 100%);
            color: @subtext_color; 
            border: 1px solid @border_color; 
            border-radius: 5px; 
            padding: 6px 10px;
            font-weight: 500;
            font-size: 0.85em;
            outline: none;
        }}
        .secondary-button:hover {{ 
            color: @text_color;
            border-color: @accent_gray;
            background: linear-gradient(180deg, #1c1c1e 0%, #121214 100%);
        }}
        .secondary-button:focus {{
            outline: none;
            box-shadow: none;
        }}
        
        .status-button {{
            background: linear-gradient(180deg, #161618 0%, #0e0e10 100%);
            color: @subtext_color;
            border: 1px solid @border_color;
            border-radius: 4px;
            padding: 4px 10px;
            font-weight: 500;
            font-size: 0.8em;
            min-height: 0;
            min-width: 0;
            outline: none;
        }}
        .status-button:hover {{
            color: @text_color;
            border-color: @accent_gray;
            background: linear-gradient(180deg, #1c1c1e 0%, #121214 100%);
        }}
        .status-button:focus {{
            outline: none;
            box-shadow: none;
        }}
        
        .icon-button {{ 
            background-color: transparent; 
            border: none; 
            border-radius: 5px; 
            padding: 6px; 
            min-width: 28px; 
            min-height: 28px;
            color: @subtext_color;
            font-size: 0.95em;
            outline: none;
        }}
        .icon-button:hover {{ 
            background-color: @overlay_color;
            color: @text_color;
        }}
        .icon-button:focus {{
            outline: none;
            box-shadow: none;
        }}
        
        .lock-subtitle {{ 
            color: @subtext_color; 
            margin-bottom: 22px; 
            font-size: 0.95em; 
        }}
        
        .password-entry {{ 
            background-color: @surface_color; 
            border: 1px solid @border_color; 
            border-radius: 6px; 
            padding: 12px 16px; 
            font-size: 1.05em; 
            min-width: 280px; 
            color: @text_color;
            outline: none;
        }}
        .password-entry:focus {{ 
            border-color: @focus_color;
            outline: none;
            box-shadow: none;
        }}
        
        .unlock-button {{ 
            background: linear-gradient(180deg, #1c1c20 0%, #101014 100%);
            color: @text_color; 
            border: 1px solid @accent_gray; 
            border-radius: 6px; 
            padding: 12px 32px; 
            font-size: 1.05em; 
            font-weight: 600; 
            margin-top: 14px;
            outline: none;
        }}
        .unlock-button:hover {{ 
            background: linear-gradient(180deg, #222226 0%, #161618 100%);
            border-color: @accent_light;
        }}
        .unlock-button:focus {{
            outline: none;
            box-shadow: none;
        }}
        
        .error-label {{ color: #a06060; font-size: 0.88em; }}
        .success-label {{ color: #60a060; font-size: 0.88em; }}
        
        .preferences-group {{ 
            background: linear-gradient(180deg, @surface_color 0%, #08080a 100%);
            border-radius: 6px; 
            padding: 12px; 
            margin: 6px 0;
            border: 1px solid @border_color;
        }}
        .preferences-title {{ 
            font-weight: 600; 
            font-size: 0.7em; 
            color: @subtext_color; 
            margin-bottom: 10px;
            text-transform: uppercase;
            letter-spacing: 1px;
        }}
        
        spinbutton {{ 
            background-color: @surface_color; 
            border: 1px solid @border_color; 
            border-radius: 4px; 
            color: @text_color;
            outline: none;
        }}
        spinbutton:focus {{
            border-color: @focus_color;
            outline: none;
            box-shadow: none;
        }}
        
        entry {{ 
            background-color: @surface_color; 
            border: 1px solid @border_color; 
            border-radius: 4px; 
            padding: 6px 8px; 
            color: @text_color;
            outline: none;
        }}
        entry:focus {{ 
            border-color: @focus_color;
            outline: none;
            box-shadow: none;
        }}
        
        checkbutton {{
            color: @text_color;
        }}
        checkbutton check {{
            background-color: @surface_color;
            border: 1px solid @border_color;
            border-radius: 3px;
        }}
        checkbutton:checked check {{
            background-color: @accent_gray;
            border-color: @accent_light;
        }}
        
        switch {{
            background-color: @surface_color;
            border: 1px solid @border_color;
        }}
        switch:checked {{
            background-color: @accent_gray;
        }}
        
        dropdown button {{
            background-color: @surface_color;
            border: 1px solid @border_color;
            color: @text_color;
            border-radius: 4px;
            padding: 4px 8px;
            outline: none;
        }}
        dropdown button:focus {{
            border-color: @focus_color;
            outline: none;
            box-shadow: none;
        }}
    "#, editor_css)
}

fn get_light_css() -> String {
    let editor_css = get_editor_font_css();
    format!(r#"
        /* Light Theme */
        @define-color bg_color #f2f2f2;
        @define-color surface_color #ffffff;
        @define-color overlay_color #e6e6e6;
        @define-color text_color #1a1a1a;
        @define-color subtext_color #555555;
        @define-color accent_gray #404040;
        @define-color accent_light #606060;
        @define-color border_color #cccccc;
        @define-color focus_color #888888;

        /* Aggressive focus removal */
        *, *:focus, *:focus-within, *:focus-visible {{
            outline: none;
            outline-width: 0;
            box-shadow: none;
        }}

        window, .background {{ background-color: @bg_color; color: @text_color; }}
        
        .custom-headerbar {{
            background: linear-gradient(180deg, #f8f8f8 0%, #e8e8e8 100%);
            border-bottom: 1px solid @border_color;
            padding: 4px 8px;
            min-height: 32px;
        }}
        
        .headerbar-title {{
            font-family: 'DotGothic16', 'Noto Sans', monospace;
            font-size: 0.95em;
            font-weight: 600;
            color: @text_color;
        }}
        
        .traffic-btn {{
            min-width: 12px;
            min-height: 12px;
            max-width: 12px;
            max-height: 12px;
            padding: 0;
            margin: 0 4px;
            border-radius: 6px;
            border: none;
            font-size: 0;
            -gtk-icon-size: 0;
        }}
        .traffic-btn:hover {{ opacity: 0.8; }}
        .traffic-close {{ background-color: #ff5f57; background-image: none; }}
        .traffic-close:hover {{ background-color: #ff3b30; background-image: none; }}
        .traffic-minimize {{ background-color: #ffbd2e; background-image: none; }}
        .traffic-minimize:hover {{ background-color: #ff9500; background-image: none; }}
        .traffic-maximize {{ background-color: #28c840; background-image: none; }}
        .traffic-maximize:hover {{ background-color: #00b341; background-image: none; }}
        
        .title-toggle {{
            min-width: 36px;
            min-height: 18px;
            border-radius: 9px;
            background-color: @overlay_color;
            border: 1px solid @border_color;
        }}
        .title-toggle:checked {{ background-color: @accent_gray; }}
        .title-toggle slider {{ min-width: 14px; min-height: 14px; border-radius: 7px; background-color: @subtext_color; }}
        .title-toggle:checked slider {{ background-color: @surface_color; }}
        
        .sidebar {{ background-color: @surface_color; border-right: 1px solid @border_color; }}
        .sidebar-header {{ padding: 10px 10px; border-bottom: 1px solid @border_color; background: transparent; }}
        
        .app-title {{ font-family: 'DotGothic16', 'Noto Sans', monospace; font-size: 1.3em; font-weight: bold; color: @text_color; }}
        .lock-title {{ font-family: 'DotGothic16', 'Noto Sans', monospace; font-size: 3.2em; font-weight: bold; color: @text_color; margin-bottom: 12px; }}
        .lock-screen {{ background-color: @bg_color; }}
        
        .search-entry {{ background-color: @bg_color; border: 1px solid @border_color; border-radius: 5px; padding: 6px 8px; margin: 6px 8px; color: @text_color; outline: none; }}
        .search-entry:focus {{ border-color: @focus_color; outline: none; box-shadow: none; }}
        
        .note-list {{ background-color: transparent; }}
        .note-list row {{ padding: 8px 10px; margin: 1px 4px; border-radius: 5px; background-color: transparent; border: 1px solid transparent; }}
        .note-list row:hover {{ background-color: @overlay_color; }}
        .note-list row:selected {{ background-color: @overlay_color; border-color: @accent_gray; }}
        
        .note-title {{ font-weight: 600; font-size: 0.9em; color: @text_color; }}
        .note-preview {{ font-size: 0.78em; color: @subtext_color; margin-top: 2px; }}
        .note-date {{ font-size: 0.7em; color: alpha(@subtext_color, 0.7); margin-top: 2px; }}
        
        .editor-area {{ background-color: @bg_color; padding: 16px; }}
        .title-entry {{ font-size: 1.3em; font-weight: bold; background-color: transparent; border: none; border-bottom: 1px solid @border_color; border-radius: 0; padding: 6px 4px; margin-bottom: 12px; color: @text_color; outline: none; }}
        .title-entry:focus {{ border-bottom-color: @focus_color; outline: none; box-shadow: none; }}
        
        .content-view {{ background-color: @surface_color; border-radius: 6px; padding: 12px; color: @text_color; border: 1px solid @border_color; outline: none; }}
        .content-view text {{ background-color: transparent; color: @text_color; }}
        .content-view:focus {{ border-color: @focus_color; outline: none; box-shadow: none; }}
        
        {}
        
        .status-bar {{ background-color: @surface_color; padding: 6px 10px; border-top: 1px solid @border_color; }}
        .status-text {{ color: @subtext_color; font-size: 0.8em; }}
        
        .action-button {{ background: @surface_color; color: @text_color; border: 1px solid @accent_gray; border-radius: 5px; padding: 7px 14px; font-weight: 600; font-size: 0.9em; outline: none; }}
        .action-button:hover {{ background: @overlay_color; }}
        .action-button:focus {{ outline: none; box-shadow: none; }}
        
        .secondary-button {{ background: @surface_color; color: @subtext_color; border: 1px solid @border_color; border-radius: 5px; padding: 6px 10px; font-size: 0.85em; outline: none; }}
        .secondary-button:hover {{ border-color: @accent_gray; color: @text_color; }}
        .secondary-button:focus {{ outline: none; box-shadow: none; }}
        
        .status-button {{ background: @surface_color; color: @subtext_color; border: 1px solid @border_color; border-radius: 4px; padding: 4px 10px; font-size: 0.8em; min-height: 0; min-width: 0; outline: none; }}
        .status-button:hover {{ border-color: @accent_gray; color: @text_color; }}
        .status-button:focus {{ outline: none; box-shadow: none; }}
        
        .icon-button {{ background-color: transparent; border: none; border-radius: 5px; padding: 6px; min-width: 28px; min-height: 28px; color: @subtext_color; font-size: 0.95em; outline: none; }}
        .icon-button:hover {{ background-color: @overlay_color; color: @text_color; }}
        .icon-button:focus {{ outline: none; box-shadow: none; }}
        
        .lock-subtitle {{ color: @subtext_color; margin-bottom: 22px; font-size: 0.95em; }}
        .password-entry {{ background-color: @surface_color; border: 1px solid @border_color; border-radius: 6px; padding: 12px 16px; font-size: 1.05em; min-width: 280px; color: @text_color; outline: none; }}
        .password-entry:focus {{ border-color: @focus_color; outline: none; box-shadow: none; }}
        
        .unlock-button {{ background: @surface_color; color: @text_color; border: 1px solid @accent_gray; border-radius: 6px; padding: 12px 32px; font-size: 1.05em; font-weight: 600; margin-top: 14px; outline: none; }}
        .unlock-button:hover {{ background: @overlay_color; }}
        .unlock-button:focus {{ outline: none; box-shadow: none; }}
        
        .error-label {{ color: #a04040; font-size: 0.88em; }}
        .success-label {{ color: #40a040; font-size: 0.88em; }}
        .preferences-group {{ background: @surface_color; border-radius: 6px; padding: 12px; margin: 6px 0; border: 1px solid @border_color; }}
        .preferences-title {{ font-weight: 600; font-size: 0.7em; color: @subtext_color; margin-bottom: 10px; text-transform: uppercase; letter-spacing: 1px; }}
        
        spinbutton {{ background-color: @surface_color; border: 1px solid @border_color; border-radius: 4px; color: @text_color; outline: none; }}
        spinbutton:focus {{ border-color: @focus_color; outline: none; box-shadow: none; }}
        entry {{ background-color: @surface_color; border: 1px solid @border_color; border-radius: 4px; padding: 6px 8px; color: @text_color; outline: none; }}
        entry:focus {{ border-color: @focus_color; outline: none; box-shadow: none; }}
        
        checkbutton {{ color: @text_color; }}
        checkbutton check {{ background-color: @surface_color; border: 1px solid @border_color; border-radius: 3px; }}
        checkbutton:checked check {{ background-color: @accent_gray; }}
        
        switch {{ background-color: @overlay_color; border: 1px solid @border_color; }}
        switch:checked {{ background-color: @accent_gray; }}
    "#, editor_css)
}

fn load_css(theme: &AppTheme) {
    CSS_PROVIDER.with(|provider_cell| {
        let provider = provider_cell.borrow_mut().get_or_insert_with(gtk::CssProvider::new).clone();
        let css = match theme {
            AppTheme::Dark => get_dark_css(),
            AppTheme::Light => get_light_css(),
        };
        provider.load_from_data(&css);
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });
}

fn switch_theme(theme: &AppTheme) {
    CURRENT_THEME.with(|t| *t.borrow_mut() = theme.clone());
    reload_css();
}

fn reload_css() {
    let theme = CURRENT_THEME.with(|t| t.borrow().clone());
    CSS_PROVIDER.with(|provider_cell| {
        if let Some(provider) = provider_cell.borrow().as_ref() {
            let css = match theme {
                AppTheme::Dark => get_dark_css(),
                AppTheme::Light => get_light_css(),
            };
            provider.load_from_data(&css);
        }
    });
}

fn reset_activity_timer() {
    LAST_ACTIVITY.with(|last| {
        *last.borrow_mut() = Instant::now();
    });
}

fn build_ui(app: &Application) {
    reset_activity_timer();
    
    if !CoreManager::is_unlocked() {
        show_password_screen(app);
    } else {
        show_main_window(app);
    }
}

fn create_traffic_light_buttons(window: &ApplicationWindow) -> gtk::Box {
    let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    btn_box.set_margin_start(4);
    
    // Close button (red)
    let close_btn = gtk::Button::new();
    close_btn.add_css_class("traffic-btn");
    close_btn.add_css_class("traffic-close");
    close_btn.set_tooltip_text(Some("Close"));
    
    // Minimize button (yellow)
    let minimize_btn = gtk::Button::new();
    minimize_btn.add_css_class("traffic-btn");
    minimize_btn.add_css_class("traffic-minimize");
    minimize_btn.set_tooltip_text(Some("Minimize"));
    
    // Maximize button (green)
    let maximize_btn = gtk::Button::new();
    maximize_btn.add_css_class("traffic-btn");
    maximize_btn.add_css_class("traffic-maximize");
    maximize_btn.set_tooltip_text(Some("Maximize"));
    
    // Connect close
    let window_clone = window.clone();
    close_btn.connect_clicked(move |_| {
        window_clone.close();
    });
    
    // Connect minimize
    let window_clone = window.clone();
    minimize_btn.connect_clicked(move |_| {
        window_clone.minimize();
    });
    
    // Connect maximize/unmaximize toggle
    let window_clone = window.clone();
    maximize_btn.connect_clicked(move |_| {
        if window_clone.is_maximized() {
            window_clone.unmaximize();
        } else {
            window_clone.maximize();
        }
    });
    
    btn_box.append(&close_btn);
    btn_box.append(&minimize_btn);
    btn_box.append(&maximize_btn);
    
    btn_box
}

fn show_password_screen(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Notas")
        .default_width(420)
        .default_height(380)
        .build();
    
    // Create custom header bar
    let header = gtk::HeaderBar::new();
    header.set_show_title_buttons(false);
    header.add_css_class("custom-headerbar");
    
    // Traffic light buttons
    let traffic_buttons = create_traffic_light_buttons(&window);
    header.pack_start(&traffic_buttons);
    
    // Title
    let header_title = Label::new(Some("Notas"));
    header_title.add_css_class("headerbar-title");
    header.set_title_widget(Some(&header_title));
    
    window.set_titlebar(Some(&header));
    window.add_css_class("lock-screen");

    let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    main_box.set_valign(gtk::Align::Center);
    main_box.set_halign(gtk::Align::Center);
    main_box.set_margin_top(20);
    main_box.set_margin_bottom(40);
    main_box.set_margin_start(40);
    main_box.set_margin_end(40);

    let title = Label::new(Some("Notas"));
    title.add_css_class("lock-title");

    let subtitle = Label::new(Some("Enter your master password"));
    subtitle.add_css_class("lock-subtitle");

    let password_entry = gtk::PasswordEntry::new();
    password_entry.set_placeholder_text(Some("Password"));
    password_entry.add_css_class("password-entry");
    password_entry.set_show_peek_icon(true);

    let status_label = Arc::new(Label::new(None));
    status_label.set_margin_top(12);

    let unlock_button = gtk::Button::with_label("Unlock");
    unlock_button.add_css_class("unlock-button");

    let window_clone = window.clone();
    let app_clone = app.clone();
    let status_label_clone = status_label.clone();
    let password_entry_clone = password_entry.clone();

    let do_unlock = move || {
        let password = password_entry_clone.text().to_string();
        if password.is_empty() {
            status_label_clone.set_markup("<span foreground='#a06060'>Password cannot be empty</span>");
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
                status_label_clone.set_markup(&format!("<span foreground='#a06060'>{}</span>", e));
            }
        };
    };

    let do_unlock_clone = do_unlock.clone();
    unlock_button.connect_clicked(move |_| {
        do_unlock_clone();
    });

    password_entry.connect_activate(move |_| {
        do_unlock();
    });

    main_box.append(&title);
    main_box.append(&subtitle);
    main_box.append(&password_entry);
    main_box.append(status_label.as_ref());
    main_box.append(&unlock_button);

    window.set_child(Some(&main_box));
    window.present();
    password_entry.grab_focus();
}

fn show_main_window(app: &Application) {
    let manager_rc = CORE_MANAGER.get().unwrap().clone();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Notas")
        .default_width(1000)
        .default_height(700)
        .build();
    
    // Create custom header bar
    let header = gtk::HeaderBar::new();
    header.set_show_title_buttons(false);
    header.add_css_class("custom-headerbar");
    
    // Traffic light buttons on left
    let traffic_buttons = create_traffic_light_buttons(&window);
    header.pack_start(&traffic_buttons);
    
    // Title in center
    let header_title = Label::new(Some("Notas"));
    header_title.add_css_class("headerbar-title");
    header.set_title_widget(Some(&header_title));
    
    window.set_titlebar(Some(&header));
    
    let motion_controller = gtk::EventControllerMotion::new();
    motion_controller.connect_motion(|_, _, _| {
        reset_activity_timer();
    });
    window.add_controller(motion_controller);

    let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    paned.set_position(220);
    paned.set_wide_handle(false);

    // Compact sidebar
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar.set_size_request(140, -1);
    sidebar.add_css_class("sidebar");

    let sidebar_header = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    sidebar_header.add_css_class("sidebar-header");
    
    let app_title = Label::new(Some("Notas"));
    app_title.add_css_class("app-title");
    app_title.set_hexpand(true);
    app_title.set_halign(gtk::Align::Start);
    
    let header_buttons = gtk::Box::new(gtk::Orientation::Horizontal, 1);
    
    let current_theme = CURRENT_THEME.with(|t| t.borrow().clone());
    let theme_button = gtk::Button::with_label(if current_theme == AppTheme::Dark { "◐" } else { "◑" });
    theme_button.add_css_class("icon-button");
    theme_button.set_tooltip_text(Some("Toggle Theme"));
    
    let settings_button = gtk::Button::with_label("⚙");
    settings_button.add_css_class("icon-button");
    settings_button.set_tooltip_text(Some("Preferences"));
    
    let lock_button = gtk::Button::with_label("⏻");
    lock_button.add_css_class("icon-button");
    lock_button.set_tooltip_text(Some("Lock"));
    
    header_buttons.append(&theme_button);
    header_buttons.append(&settings_button);
    header_buttons.append(&lock_button);
    
    sidebar_header.append(&app_title);
    sidebar_header.append(&header_buttons);

    let search_entry = gtk::SearchEntry::new();
    search_entry.set_placeholder_text(Some("Search..."));
    search_entry.add_css_class("search-entry");

    let new_note_button = gtk::Button::with_label("+ New Note");
    new_note_button.add_css_class("action-button");
    new_note_button.set_margin_start(8);
    new_note_button.set_margin_end(8);
    new_note_button.set_margin_top(2);
    new_note_button.set_margin_bottom(6);

    let note_list_box = Arc::new(gtk::ListBox::new());
    note_list_box.set_selection_mode(gtk::SelectionMode::Single);
    note_list_box.add_css_class("note-list");

    let scrolled_window = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(note_list_box.as_ref())
        .vexpand(true)
        .build();

    let sidebar_footer = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    sidebar_footer.set_margin_start(8);
    sidebar_footer.set_margin_end(8);
    sidebar_footer.set_margin_top(4);
    sidebar_footer.set_margin_bottom(6);
    
    let export_button = gtk::Button::with_label("Export");
    export_button.add_css_class("secondary-button");
    export_button.set_hexpand(true);
    
    let import_button = gtk::Button::with_label("Import");
    import_button.add_css_class("secondary-button");
    import_button.set_hexpand(true);
    
    sidebar_footer.append(&export_button);
    sidebar_footer.append(&import_button);

    sidebar.append(&sidebar_header);
    sidebar.append(&search_entry);
    sidebar.append(&new_note_button);
    sidebar.append(&scrolled_window);
    sidebar.append(&sidebar_footer);

    let editor_area = gtk::Box::new(gtk::Orientation::Vertical, 0);
    editor_area.add_css_class("editor-area");
    editor_area.set_hexpand(true);
    editor_area.set_size_request(300, -1);

    // Title entry - can be hidden
    let title_entry = Arc::new(gtk::Entry::new());
    title_entry.set_placeholder_text(Some("Note Title"));
    title_entry.add_css_class("title-entry");
    
    // Set initial visibility based on settings
    let show_title = SHOW_NOTE_TITLE.with(|s| *s.borrow());
    title_entry.set_visible(show_title);

    let content_buffer = Arc::new(gtk::TextBuffer::new(None));
    let content_view = gtk::TextView::builder()
        .buffer(content_buffer.as_ref())
        .vexpand(true)
        .editable(true)
        .wrap_mode(gtk::WrapMode::Word)
        .left_margin(10)
        .right_margin(10)
        .top_margin(10)
        .bottom_margin(10)
        .build();
    content_view.add_css_class("content-view");

    let editor_scrolled_window = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&content_view)
        .vexpand(true)
        .build();

    let status_bar = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    status_bar.add_css_class("status-bar");
    
    // Title toggle switch
    let title_toggle = gtk::Switch::new();
    title_toggle.add_css_class("title-toggle");
    title_toggle.set_active(SHOW_NOTE_TITLE.with(|s| *s.borrow()));
    title_toggle.set_tooltip_text(Some("Show/hide title"));
    
    let title_toggle_label = Label::new(Some(""));
    title_toggle_label.add_css_class("status-text");
    title_toggle_label.set_tooltip_text(Some("Toggle title"));
    
    let status_label = Arc::new(Label::new(Some("")));
    status_label.add_css_class("status-text");
    status_label.set_hexpand(true);
    status_label.set_halign(gtk::Align::Start);

    let copy_button = Arc::new(gtk::Button::with_label("Copy"));
    copy_button.add_css_class("status-button");
    copy_button.set_sensitive(false);

    let save_button = Arc::new(gtk::Button::with_label("Save"));
    save_button.add_css_class("status-button");
    save_button.set_sensitive(false);

    let delete_button = Arc::new(gtk::Button::with_label("Delete"));
    delete_button.add_css_class("status-button");
    delete_button.set_sensitive(false);

    status_bar.append(&title_toggle_label);
    status_bar.append(&title_toggle);
    status_bar.append(status_label.as_ref());
    status_bar.append(copy_button.as_ref());
    status_bar.append(save_button.as_ref());
    status_bar.append(delete_button.as_ref());
    
    // Connect title toggle
    let title_entry_for_toggle = title_entry.clone();
    let manager_for_toggle = manager_rc.clone();
    title_toggle.connect_state_set(move |_, state| {
        title_entry_for_toggle.set_visible(state);
        SHOW_NOTE_TITLE.with(|s| *s.borrow_mut() = state);
        
        // Save setting
        let mut settings = manager_for_toggle.lock().unwrap().get_settings().clone();
        settings.show_note_title = state;
        let _ = manager_for_toggle.lock().unwrap().update_settings(settings);
        
        glib::Propagation::Proceed
    });

    editor_area.append(title_entry.as_ref());
    editor_area.append(&editor_scrolled_window);
    editor_area.append(&status_bar);

    paned.set_start_child(Some(&sidebar));
    paned.set_end_child(Some(&editor_area));
    paned.set_resize_start_child(true);
    paned.set_shrink_start_child(true);
    paned.set_resize_end_child(true);
    paned.set_shrink_end_child(false);

    window.set_child(Some(&paned));

    let active_note_id = Arc::new(Mutex::new(None::<u64>));
    let row_ids: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));
    let search_text: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    // Set to true just before select_row on a newly created note so the
    // row_selected handler doesn't overwrite the blank title entry with "Untitled".
    let skip_next_load: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    // Set to true whenever we programmatically clear the editor (new note, delete,
    // row selection) so connect_changed doesn't mistake it for the user typing and
    // spawn a phantom "Untitled" note.
    let suppress_auto_create: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));

    let refresh_note_list = {
        let note_list_box = note_list_box.clone();
        let manager_rc = manager_rc.clone();
        let row_ids = row_ids.clone();
        let search_text = search_text.clone();

        move || {
            while let Some(child) = note_list_box.first_child() {
                note_list_box.remove(&child);
            }
            row_ids.lock().unwrap().clear();

            let notes = manager_rc.lock().unwrap().get_notes();
            let search = search_text.lock().unwrap().to_lowercase();
            
            for note in notes {
                if !search.is_empty() {
                    let title_lower = note.title.to_lowercase();
                    let content_lower = note.content.to_lowercase();
                    if !title_lower.contains(&search) && !content_lower.contains(&search) {
                        continue;
                    }
                }
                
                let row = ListBoxRow::new();
                let row_box = gtk::Box::new(gtk::Orientation::Vertical, 1);
                row_box.set_margin_top(2);
                row_box.set_margin_bottom(2);

                let title_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
                
                if note.pinned {
                    let pin_icon = Label::new(Some("●"));
                    pin_icon.add_css_class("note-pinned");
                    title_box.append(&pin_icon);
                }
                
                let title_label = Label::new(Some(&note.title));
                title_label.set_halign(gtk::Align::Start);
                title_label.add_css_class("note-title");
                title_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                title_label.set_hexpand(true);
                title_box.append(&title_label);

                let preview = note.content.lines().next().unwrap_or("").chars().take(40).collect::<String>();
                let preview_label = Label::new(Some(&preview));
                preview_label.set_halign(gtk::Align::Start);
                preview_label.add_css_class("note-preview");
                preview_label.set_ellipsize(gtk::pango::EllipsizeMode::End);

                let date_label = Label::new(Some(&note.updated_at.format("%b %d, %Y").to_string()));
                date_label.set_halign(gtk::Align::Start);
                date_label.add_css_class("note-date");

                row_box.append(&title_box);
                row_box.append(&preview_label);
                row_box.append(&date_label);

                row.set_child(Some(&row_box));
                note_list_box.append(&row);
                row_ids.lock().unwrap().push(note.id);
            }
            note_list_box.show();
        }
    };

    refresh_note_list();

    let refresh_clone = refresh_note_list.clone();
    let search_text_clone = search_text.clone();
    search_entry.connect_search_changed(move |entry| {
        *search_text_clone.lock().unwrap() = entry.text().to_string();
        refresh_clone();
    });

    let manager_clone = manager_rc.clone();
    theme_button.connect_clicked(move |btn| {
        let new_theme = CURRENT_THEME.with(|t| {
            match *t.borrow() {
                AppTheme::Dark => AppTheme::Light,
                AppTheme::Light => AppTheme::Dark,
            }
        });
        switch_theme(&new_theme);
        btn.set_label(if new_theme == AppTheme::Dark { "◐" } else { "◑" });
        
        let mut settings = manager_clone.lock().unwrap().get_settings().clone();
        settings.theme = new_theme;
        let _ = manager_clone.lock().unwrap().update_settings(settings);
    });

    let window_clone = window.clone();
    let app_clone = app.clone();
    lock_button.connect_clicked(move |_| {
        let manager_rc = CORE_MANAGER.get().unwrap().clone();
        manager_rc.lock().unwrap().lock();
        window_clone.close();
        show_password_screen(&app_clone);
    });

    let window_clone = window.clone();
    let manager_clone = manager_rc.clone();
    let status_clone = status_label.clone();
    let title_entry_for_prefs = title_entry.clone();
    let title_toggle_for_prefs = title_toggle.clone();
    settings_button.connect_clicked(move |_| {
        show_preferences_dialog(&window_clone, manager_clone.clone(), status_clone.clone(), title_entry_for_prefs.clone(), title_toggle_for_prefs.clone());
    });

    // Auto-create a note if the user starts typing directly into the editor
    // without clicking "+ New Note" first. Without this, the save button stays
    // greyed out and the user's text has nowhere to go.
    let auto_create = {
        let manager_rc = manager_rc.clone();
        let active_note_id = active_note_id.clone();
        let delete_button = delete_button.clone();
        let save_button = save_button.clone();
        let copy_button = copy_button.clone();
        let status_label = status_label.clone();
        let note_list_box = note_list_box.clone();
        let skip_next_load = skip_next_load.clone();
        let suppress_auto_create = suppress_auto_create.clone();
        let refresh = refresh_note_list.clone();
        move || {
            // Don't fire if we're programmatically clearing the editor
            if *suppress_auto_create.lock().unwrap() {
                return;
            }
            // Only fire if there is no active note
            if active_note_id.lock().unwrap().is_some() {
                return;
            }
            let manager_clone = manager_rc.clone();
            let active_clone = active_note_id.clone();
            let delete_clone = delete_button.clone();
            let save_clone = save_button.clone();
            let copy_clone = copy_button.clone();
            let status_clone = status_label.clone();
            let list_box = note_list_box.clone();
            let skip_flag = skip_next_load.clone();
            let refresh = refresh.clone();

            let (sender, receiver) = async_channel::unbounded();
            let runtime = TOKIO_RUNTIME.get().unwrap();
            let manager_for_task = manager_clone.clone();
            glib::spawn_future_local(async move {
                let _guard = runtime.enter();
                let result = tokio::task::spawn_blocking(move || {
                    manager_for_task.lock().unwrap().create_note("Untitled".to_string(), String::new())
                }).await;
                let _ = sender.send(result).await;
            });
            let active_note_id_inner = active_clone.clone();
            glib::spawn_future_local(async move {
                if let Ok(Ok(Ok(new_id))) = receiver.recv().await {
                    *active_note_id_inner.lock().unwrap() = Some(new_id);
                    delete_clone.set_sensitive(true);
                    save_clone.set_sensitive(true);
                    copy_clone.set_sensitive(true);
                    status_clone.set_text("New note — enter a title and save");
                    refresh();
                    *skip_flag.lock().unwrap() = true;
                    if let Some(row) = list_box.row_at_index(0) {
                        list_box.select_row(Some(&row));
                    }
                }
            });
        }
    };

    let auto_create_for_title = auto_create.clone();
    title_entry.connect_changed(move |_| {
        auto_create_for_title();
    });

    let auto_create_for_content = auto_create.clone();
    content_buffer.connect_changed(move |_| {
        auto_create_for_content();
    });

    note_list_box.connect_row_selected(glib::clone!(@strong manager_rc, @strong title_entry, 
        @strong content_buffer, @strong delete_button, @strong save_button, @strong copy_button, 
        @strong active_note_id, @strong status_label, @strong row_ids, @strong skip_next_load,
        @strong suppress_auto_create => move |_, row_opt| {
        if let Some(row) = row_opt {
            let idx = row.index();
            if idx >= 0 {
                let id_opt = row_ids.lock().unwrap().get(idx as usize).copied();
                if let Some(id) = id_opt {
                    let skip = {
                        let mut guard = skip_next_load.lock().unwrap();
                        let val = *guard;
                        *guard = false;
                        val
                    };
                    if skip {
                        *active_note_id.lock().unwrap() = Some(id);
                        delete_button.set_sensitive(true);
                        save_button.set_sensitive(true);
                        copy_button.set_sensitive(true);
                        status_label.set_text("New note — enter a title and save");
                    } else {
                        let note_opt = manager_rc.lock().unwrap().get_notes().into_iter().find(|n| n.id == id);
                        if let Some(note) = note_opt {
                            *suppress_auto_create.lock().unwrap() = true;
                            title_entry.set_text(&note.title);
                            content_buffer.set_text(&note.content);
                            *suppress_auto_create.lock().unwrap() = false;
                            *active_note_id.lock().unwrap() = Some(id);
                            delete_button.set_sensitive(true);
                            save_button.set_sensitive(true);
                            copy_button.set_sensitive(true);
                            status_label.set_text(&format!("Editing: {}", note.title));
                        }
                    }
                }
            }
        }
        // NOTE: We intentionally do NOT disable buttons or clear active_note_id
        // when row_opt is None, because this can happen when focus moves to 
        // the paned resize handle or other widgets. The active note should
        // persist until explicitly changed (new note, delete, or selecting another note).
    }));

    let content_buffer_clone = content_buffer.clone();
    let status_label_clone = status_label.clone();
    let manager_clone = manager_rc.clone();
    copy_button.connect_clicked(move |_| {
        let content = content_buffer_clone.text(
            &content_buffer_clone.start_iter(), 
            &content_buffer_clone.end_iter(), 
            false
        ).to_string();
        
        if let Some(display) = gtk::gdk::Display::default() {
            let clipboard = display.clipboard();
            clipboard.set_text(&content);
            status_label_clone.set_text("Copied to clipboard");
            
            let timeout = manager_clone.lock().unwrap().get_settings().clipboard_timeout;
            
            if timeout > 0 {
                CLIPBOARD_TIMER.with(|timer_cell| {
                    if let Some(old_id) = timer_cell.borrow_mut().take() {
                        old_id.remove();
                    }
                });
                
                let status_for_timer = status_label_clone.clone();
                
                let timer_id = glib::timeout_add_seconds_local(timeout as u32, move || {
                    if let Some(disp) = gtk::gdk::Display::default() {
                        disp.clipboard().set_text("");
                    }
                    status_for_timer.set_text("Clipboard cleared");
                    
                    CLIPBOARD_TIMER.with(|timer_cell| {
                        *timer_cell.borrow_mut() = None;
                    });
                    
                    glib::ControlFlow::Break
                });
                
                CLIPBOARD_TIMER.with(|timer_cell| {
                    *timer_cell.borrow_mut() = Some(timer_id);
                });
            }
        }
    });

    let refresh_clone = refresh_note_list.clone();
    new_note_button.connect_clicked(glib::clone!(@strong manager_rc, @strong title_entry, 
        @strong content_buffer, @strong active_note_id, @strong delete_button, @strong save_button, 
        @strong copy_button, @strong note_list_box, @strong status_label, @strong skip_next_load,
        @strong suppress_auto_create => move |_| {
        
        // Suppress connect_changed during programmatic clear so no phantom note is created
        *suppress_auto_create.lock().unwrap() = true;
        title_entry.set_text("");
        content_buffer.set_text("");
        *suppress_auto_create.lock().unwrap() = false;

        *active_note_id.lock().unwrap() = None;
        delete_button.set_sensitive(false);
        save_button.set_sensitive(false);
        copy_button.set_sensitive(false);

        let manager_clone = manager_rc.clone();
        let status_clone = status_label.clone();
        let refresh = refresh_clone.clone();
        let list_box = note_list_box.clone();
        let skip_flag = skip_next_load.clone();

        status_label.set_text("Creating...");

        let (sender, receiver) = async_channel::unbounded();
        let runtime = TOKIO_RUNTIME.get().unwrap();
        
        let manager_for_task = manager_clone.clone();
        glib::spawn_future_local(async move {
            let _guard = runtime.enter();
            let result = tokio::task::spawn_blocking(move || {
                manager_for_task.lock().unwrap().create_note("Untitled".to_string(), String::new())
            }).await;
            let _ = sender.send(result).await;
        });

        glib::spawn_future_local(async move {
            if let Ok(result) = receiver.recv().await {
                match result {
                    Ok(Ok(new_id)) => {
                        status_clone.set_text("New note — enter a title and save");
                        refresh();
                        // Set the flag BEFORE select_row so row_selected knows
                        // not to overwrite the blank title entry with "Untitled".
                        *skip_flag.lock().unwrap() = true;
                        if let Some(row) = list_box.row_at_index(0) {
                            list_box.select_row(Some(&row));
                        }
                        // active_note_id is set inside row_selected when skip is true,
                        // but set it here too as a safety net in case select_row
                        // doesn't fire (e.g. row was already selected).
                        // The row_selected handler will overwrite this with the same value.
                        let _ = new_id; // used via skip path in row_selected
                    },
                    Ok(Err(e)) => status_clone.set_text(&format!("Error: {}", e)),
                    Err(e) => status_clone.set_text(&format!("Error: {}", e)),
                }
            }
        });
    }));

    let refresh_clone = refresh_note_list.clone();
    save_button.connect_clicked(glib::clone!(@strong manager_rc, @strong active_note_id, 
        @strong title_entry, @strong content_buffer, @strong status_label => move |_| {
        
        let id_opt = *active_note_id.lock().unwrap();
        if let Some(id) = id_opt {
            let title = title_entry.text().to_string();
            let content = content_buffer.text(
                &content_buffer.start_iter(), 
                &content_buffer.end_iter(), 
                false
            ).to_string();

            let manager_clone = manager_rc.clone();
            let status_clone = status_label.clone();
            let refresh = refresh_clone.clone();

            status_label.set_text("Saving...");

            let (sender, receiver) = async_channel::unbounded();
            let runtime = TOKIO_RUNTIME.get().unwrap();
            
            let manager_for_task = manager_clone.clone();
            glib::spawn_future_local(async move {
                let _guard = runtime.enter();
                let result = tokio::task::spawn_blocking(move || {
                    manager_for_task.lock().unwrap().update_note(id, title, content, None)
                }).await;
                let _ = sender.send(result).await;
            });

            glib::spawn_future_local(async move {
                if let Ok(result) = receiver.recv().await {
                    match result {
                        Ok(Ok(_)) => { status_clone.set_text("Saved"); refresh(); },
                        Ok(Err(e)) => status_clone.set_text(&format!("Error: {}", e)),
                        Err(e) => status_clone.set_text(&format!("Error: {}", e)),
                    }
                }
            });
        }
    }));

    let refresh_clone = refresh_note_list.clone();
    let delete_note_handler = {
        let manager_rc = manager_rc.clone();
        let active_note_id = active_note_id.clone();
        let title_entry = title_entry.clone();
        let content_buffer = content_buffer.clone();
        let delete_button = delete_button.clone();
        let save_button = save_button.clone();
        let copy_button = copy_button.clone();
        let status_label = status_label.clone();
        let refresh_clone = refresh_clone.clone();
        let suppress_auto_create = suppress_auto_create.clone();
        
        move || {
            let id_opt = *active_note_id.lock().unwrap();
            if let Some(id) = id_opt {
                let manager_clone = manager_rc.clone();
                let status_clone = status_label.clone();
                let title_clone = title_entry.clone();
                let content_clone = content_buffer.clone();
                let active_clone = active_note_id.clone();
                let delete_clone = delete_button.clone();
                let save_clone = save_button.clone();
                let copy_clone = copy_button.clone();
                let refresh = refresh_clone.clone();
                let suppress_clone = suppress_auto_create.clone();

                status_label.set_text("Deleting...");
                delete_button.set_sensitive(false);

                let (sender, receiver) = async_channel::unbounded();
                let runtime = TOKIO_RUNTIME.get().unwrap();
                
                let manager_for_task = manager_clone.clone();
                glib::spawn_future_local(async move {
                    let _guard = runtime.enter();
                    let result = tokio::task::spawn_blocking(move || {
                        manager_for_task.lock().unwrap().delete_note(id)
                    }).await;
                    let _ = sender.send(result).await;
                });

                glib::spawn_future_local(async move {
                    if let Ok(result) = receiver.recv().await {
                        match result {
                            Ok(Ok(_)) => {
                                status_clone.set_text("Deleted");
                                *suppress_clone.lock().unwrap() = true;
                                title_clone.set_text("");
                                content_clone.set_text("");
                                *suppress_clone.lock().unwrap() = false;
                                *active_clone.lock().unwrap() = None;
                                delete_clone.set_sensitive(false);
                                save_clone.set_sensitive(false);
                                copy_clone.set_sensitive(false);
                                refresh();
                            },
                            Ok(Err(e)) => { 
                                status_clone.set_text(&format!("Error: {}", e)); 
                                delete_clone.set_sensitive(true); 
                            },
                            Err(e) => { 
                                status_clone.set_text(&format!("Error: {}", e)); 
                                delete_clone.set_sensitive(true); 
                            },
                        }
                    }
                });
            }
        }
    };
    
    let delete_handler_clone = delete_note_handler.clone();
    delete_button.connect_clicked(move |_| { delete_handler_clone(); });

    let key_controller = gtk::EventControllerKey::new();
    let delete_handler_for_key = delete_note_handler.clone();
    let active_note_id_for_key = active_note_id.clone();
    let title_entry_for_key = title_entry.clone();
    let content_view_clone = content_view.clone();
    
    key_controller.connect_key_pressed(move |_, keyval, _, _| {
        reset_activity_timer();
        if keyval == gtk::gdk::Key::Delete {
            let has_active = active_note_id_for_key.lock().unwrap().is_some();
            if has_active && !title_entry_for_key.has_focus() && !content_view_clone.has_focus() {
                delete_handler_for_key();
                return glib::Propagation::Stop;
            }
        }
        glib::Propagation::Proceed
    });
    window.add_controller(key_controller);

    export_button.connect_clicked(glib::clone!(@strong manager_rc, @strong status_label, 
        @strong window => move |_| {
        let file_chooser = gtk::FileChooserDialog::new(
            Some("Export Notes"), 
            Some(&window), 
            gtk::FileChooserAction::Save,
            &[("Cancel", gtk::ResponseType::Cancel), ("Export", gtk::ResponseType::Accept)],
        );
        file_chooser.set_current_name("notes_export.dat");
        
        let manager_clone = manager_rc.clone();
        let status_clone = status_label.clone();
        
        file_chooser.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                if let Some(path) = dialog.file().and_then(|f| f.path()) {
                    let manager_for_task = manager_clone.clone();
                    let status_for_ui = status_clone.clone();
                    let path_clone = path.clone();
                    
                    status_clone.set_text("Exporting...");
                    
                    let (sender, receiver) = async_channel::unbounded();
                    let runtime = TOKIO_RUNTIME.get().unwrap();
                    
                    glib::spawn_future_local(async move {
                        let _guard = runtime.enter();
                        let result = tokio::task::spawn_blocking(move || {
                            manager_for_task.lock().unwrap().export_all_encrypted(&path_clone)
                        }).await;
                        let _ = sender.send((result, path)).await;
                    });
                    
                    glib::spawn_future_local(async move {
                        if let Ok((result, path)) = receiver.recv().await {
                            match result {
                                Ok(Ok(_)) => status_for_ui.set_text(&format!("Exported: {}", path.display())),
                                Ok(Err(e)) => status_for_ui.set_text(&format!("Error: {}", e)),
                                Err(e) => status_for_ui.set_text(&format!("Error: {}", e)),
                            }
                        }
                    });
                }
            }
            dialog.close();
        });
        file_chooser.show();
    }));

    let refresh_clone = refresh_note_list.clone();
    import_button.connect_clicked(glib::clone!(@strong manager_rc, @strong status_label, 
        @strong window => move |_| {
        let file_chooser = gtk::FileChooserDialog::new(
            Some("Import Notes"), 
            Some(&window), 
            gtk::FileChooserAction::Open,
            &[("Cancel", gtk::ResponseType::Cancel), ("Import", gtk::ResponseType::Accept)],
        );
        
        let manager_clone = manager_rc.clone();
        let status_clone = status_label.clone();
        let window_clone = window.clone();
        let refresh = refresh_clone.clone();
        
        file_chooser.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                if let Some(path) = dialog.file().and_then(|f| f.path()) {
                    show_import_password_dialog(
                        &window_clone, 
                        manager_clone.clone(), 
                        status_clone.clone(), 
                        refresh.clone(), 
                        path
                    );
                }
            }
            dialog.close();
        });
        file_chooser.show();
    }));

    let window_weak = window.downgrade();
    let app_weak = app.downgrade();
    let manager_for_timer = manager_rc.clone();
    
    glib::timeout_add_seconds_local(30, move || {
        let timeout = manager_for_timer.lock()
            .map(|m| m.get_settings().auto_lock_timeout)
            .unwrap_or(0);
        
        if timeout == 0 { 
            return glib::ControlFlow::Continue; 
        }
        
        let elapsed = LAST_ACTIVITY.with(|l| l.borrow().elapsed());
        
        if elapsed >= Duration::from_secs(timeout) {
            if let (Some(w), Some(a)) = (window_weak.upgrade(), app_weak.upgrade()) {
                if let Some(mgr) = CORE_MANAGER.get() {
                    mgr.clone().lock().unwrap().lock();
                }
                w.close();
                show_password_screen(&a);
                return glib::ControlFlow::Break;
            }
        }
        glib::ControlFlow::Continue
    });

    window.present();
}

fn show_preferences_dialog(
    parent: &ApplicationWindow,
    manager_rc: Arc<Mutex<CoreManager>>,
    status_label: Arc<Label>,
    title_entry_widget: Arc<gtk::Entry>,
    title_toggle_widget: gtk::Switch,
) {
    let settings = manager_rc.lock().unwrap().get_settings().clone();
    let db_path = manager_rc.lock().unwrap().get_data_path().display().to_string();
    
    let dialog = gtk::Window::builder()
        .title("Preferences")
        .modal(true)
        .transient_for(parent)
        .default_width(420)
        .default_height(560)
        .build();
    
    // Custom header for dialog too
    let header = gtk::HeaderBar::new();
    header.set_show_title_buttons(false);
    header.add_css_class("custom-headerbar");
    
    let dialog_for_header = dialog.clone();
    let close_btn = gtk::Button::new();
    close_btn.add_css_class("traffic-btn");
    close_btn.add_css_class("traffic-close");
    close_btn.connect_clicked(move |_| dialog_for_header.close());
    
    let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    btn_box.set_margin_start(4);
    btn_box.append(&close_btn);
    header.pack_start(&btn_box);
    
    let header_title = Label::new(Some("Preferences"));
    header_title.add_css_class("headerbar-title");
    header.set_title_widget(Some(&header_title));
    
    dialog.set_titlebar(Some(&header));

    let scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();

    let main_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
    main_box.set_margin_top(18);
    main_box.set_margin_bottom(18);
    main_box.set_margin_start(18);
    main_box.set_margin_end(18);

    // Editor group
    let editor_group = gtk::Box::new(gtk::Orientation::Vertical, 8);
    editor_group.add_css_class("preferences-group");
    
    let editor_title = Label::new(Some("EDITOR"));
    editor_title.add_css_class("preferences-title");
    editor_title.set_halign(gtk::Align::Start);
    
    // Font family dropdown
    let font_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let font_label = Label::new(Some("Font"));
    font_label.set_hexpand(true);
    font_label.set_halign(gtk::Align::Start);
    
    let font_dropdown = gtk::DropDown::from_strings(
        &EditorFont::all_fonts().iter().map(|f| f.display_name()).collect::<Vec<_>>()
    );
    font_dropdown.set_selected(settings.editor_font.to_index());
    
    font_row.append(&font_label);
    font_row.append(&font_dropdown);
    
    // Font size
    let size_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let size_label = Label::new(Some("Font size"));
    size_label.set_hexpand(true);
    size_label.set_halign(gtk::Align::Start);
    let size_spin = gtk::SpinButton::with_range(6.0, 24.0, 1.0);
    size_spin.set_value(settings.editor_font_size as f64);
    size_row.append(&size_label);
    size_row.append(&size_spin);
    
    // Show title toggle
    let title_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let title_label = Label::new(Some("Show note title"));
    title_label.set_hexpand(true);
    title_label.set_halign(gtk::Align::Start);
    let title_switch = gtk::Switch::new();
    title_switch.set_active(settings.show_note_title);
    title_row.append(&title_label);
    title_row.append(&title_switch);
    
    editor_group.append(&editor_title);
    editor_group.append(&font_row);
    editor_group.append(&size_row);
    editor_group.append(&title_row);

    // Security group
    let security_group = gtk::Box::new(gtk::Orientation::Vertical, 8);
    security_group.add_css_class("preferences-group");
    
    let security_title = Label::new(Some("SECURITY"));
    security_title.add_css_class("preferences-title");
    security_title.set_halign(gtk::Align::Start);
    
    let auto_lock_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let auto_lock_label = Label::new(Some("Auto-lock (sec, 0=off)"));
    auto_lock_label.set_hexpand(true);
    auto_lock_label.set_halign(gtk::Align::Start);
    let auto_lock_spin = gtk::SpinButton::with_range(0.0, 3600.0, 30.0);
    auto_lock_spin.set_value(settings.auto_lock_timeout as f64);
    auto_lock_row.append(&auto_lock_label);
    auto_lock_row.append(&auto_lock_spin);
    
    let clipboard_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let clipboard_label = Label::new(Some("Clipboard clear (sec, 0=off)"));
    clipboard_label.set_hexpand(true);
    clipboard_label.set_halign(gtk::Align::Start);
    let clipboard_spin = gtk::SpinButton::with_range(0.0, 300.0, 5.0);
    clipboard_spin.set_value(settings.clipboard_timeout as f64);
    clipboard_row.append(&clipboard_label);
    clipboard_row.append(&clipboard_spin);
    
    security_group.append(&security_title);
    security_group.append(&auto_lock_row);
    security_group.append(&clipboard_row);

    // Storage group
    let storage_group = gtk::Box::new(gtk::Orientation::Vertical, 8);
    storage_group.add_css_class("preferences-group");
    
    let storage_title = Label::new(Some("STORAGE"));
    storage_title.add_css_class("preferences-title");
    storage_title.set_halign(gtk::Align::Start);
    
    let path_label = Label::new(Some("Database location:"));
    path_label.set_halign(gtk::Align::Start);
    
    let path_entry = gtk::Entry::new();
    path_entry.set_text(&db_path);
    path_entry.set_hexpand(true);
    
    storage_group.append(&storage_title);
    storage_group.append(&path_label);
    storage_group.append(&path_entry);

    // Password group
    let password_group = gtk::Box::new(gtk::Orientation::Vertical, 8);
    password_group.add_css_class("preferences-group");
    
    let password_title = Label::new(Some("CHANGE PASSWORD"));
    password_title.add_css_class("preferences-title");
    password_title.set_halign(gtk::Align::Start);
    
    let current_password_entry = gtk::PasswordEntry::new();
    current_password_entry.set_placeholder_text(Some("Current Password"));
    current_password_entry.set_show_peek_icon(true);
    
    let new_password_entry = gtk::PasswordEntry::new();
    new_password_entry.set_placeholder_text(Some("New Password"));
    new_password_entry.set_show_peek_icon(true);
    
    let confirm_password_entry = gtk::PasswordEntry::new();
    confirm_password_entry.set_placeholder_text(Some("Confirm New Password"));
    confirm_password_entry.set_show_peek_icon(true);
    
    let change_password_button = gtk::Button::with_label("Change Password");
    change_password_button.add_css_class("secondary-button");
    
    let password_status = Rc::new(Label::new(None));
    password_status.set_halign(gtk::Align::Start);
    
    password_group.append(&password_title);
    password_group.append(&current_password_entry);
    password_group.append(&new_password_entry);
    password_group.append(&confirm_password_entry);
    password_group.append(&change_password_button);
    password_group.append(password_status.as_ref());

    let manager_clone = manager_rc.clone();
    let current_clone = current_password_entry.clone();
    let new_clone = new_password_entry.clone();
    let confirm_clone = confirm_password_entry.clone();
    let password_status_clone = password_status.clone();
    
    change_password_button.connect_clicked(move |btn| {
        let current = current_clone.text().to_string();
        let new_pass = new_clone.text().to_string();
        let confirm = confirm_clone.text().to_string();
        
        if current.is_empty() || new_pass.is_empty() || confirm.is_empty() {
            password_status_clone.set_markup("<span foreground='#a06060'>All fields required</span>");
            return;
        }
        if new_pass != confirm {
            password_status_clone.set_markup("<span foreground='#a06060'>Passwords don't match</span>");
            return;
        }
        if new_pass.len() < 8 {
            password_status_clone.set_markup("<span foreground='#a06060'>Min 8 characters</span>");
            return;
        }
        
        btn.set_sensitive(false);
        password_status_clone.set_text("Changing...");
        
        let manager_for_task = manager_clone.clone();
        let password_status_for_ui = password_status_clone.clone();
        let current_entry = current_clone.clone();
        let new_entry = new_clone.clone();
        let confirm_entry = confirm_clone.clone();
        let btn_ui = btn.clone();
        
        let (sender, receiver) = async_channel::unbounded();
        let runtime = TOKIO_RUNTIME.get().unwrap();
        
        glib::spawn_future_local(async move {
            let _guard = runtime.enter();
            let result = tokio::task::spawn_blocking(move || {
                let old = core::data::MasterPassword::from(current.as_str());
                let new = core::data::MasterPassword::from(new_pass.as_str());
                manager_for_task.lock().unwrap().change_password(old, new)
            }).await;
            let _ = sender.send(result).await;
        });
        
        glib::spawn_future_local(async move {
            if let Ok(result) = receiver.recv().await {
                match result {
                    Ok(Ok(_)) => {
                        password_status_for_ui.set_markup("<span foreground='#60a060'>Password changed</span>");
                        current_entry.set_text("");
                        new_entry.set_text("");
                        confirm_entry.set_text("");
                    },
                    Ok(Err(e)) => password_status_for_ui.set_markup(&format!("<span foreground='#a06060'>{}</span>", e)),
                    Err(e) => password_status_for_ui.set_markup(&format!("<span foreground='#a06060'>{}</span>", e)),
                }
                btn_ui.set_sensitive(true);
            }
        });
    });

    let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    button_box.set_halign(gtk::Align::End);
    button_box.set_margin_top(8);
    
    let cancel_button = gtk::Button::with_label("Cancel");
    cancel_button.add_css_class("secondary-button");
    
    let save_button = gtk::Button::with_label("Save");
    save_button.add_css_class("action-button");
    
    button_box.append(&cancel_button);
    button_box.append(&save_button);

    main_box.append(&editor_group);
    main_box.append(&security_group);
    main_box.append(&storage_group);
    main_box.append(&password_group);
    main_box.append(&button_box);

    scrolled.set_child(Some(&main_box));
    dialog.set_child(Some(&scrolled));

    let dialog_clone = dialog.clone();
    cancel_button.connect_clicked(move |_| { dialog_clone.close(); });

    let dialog_clone = dialog.clone();
    let manager_clone = manager_rc.clone();
    let path_entry_clone = path_entry.clone();
    let font_dropdown_clone = font_dropdown.clone();
    let size_spin_clone = size_spin.clone();
    let title_switch_clone = title_switch.clone();
    let title_entry_clone = title_entry_widget.clone();
    let title_toggle_clone = title_toggle_widget.clone();
    
    save_button.connect_clicked(move |_| {
        let current = manager_clone.lock().unwrap().get_settings().clone();
        let default_path = manager_clone.lock().unwrap().get_data_path().display().to_string();
        let path_str = path_entry_clone.text().to_string();
        
        let selected_font = EditorFont::from_index(font_dropdown_clone.selected());
        let font_size = size_spin_clone.value() as u32;
        let show_title = title_switch_clone.is_active();
        
        // Update thread-local settings
        EDITOR_FONT.with(|f| *f.borrow_mut() = selected_font.clone());
        EDITOR_FONT_SIZE.with(|s| *s.borrow_mut() = font_size);
        SHOW_NOTE_TITLE.with(|s| *s.borrow_mut() = show_title);
        
        // Update title visibility immediately and keep status bar toggle in sync
        title_entry_clone.set_visible(show_title);
        title_toggle_clone.set_active(show_title);
        
        let new_settings = AppSettings {
            auto_lock_timeout: auto_lock_spin.value() as u64,
            clipboard_timeout: clipboard_spin.value() as u64,
            custom_db_path: if path_str != default_path { 
                Some(std::path::PathBuf::from(path_str)) 
            } else { 
                None 
            },
            argon2_params: current.argon2_params,
            theme: CURRENT_THEME.with(|t| t.borrow().clone()),
            editor_font: selected_font,
            editor_font_size: font_size,
            show_note_title: show_title,
        };
        
        match manager_clone.lock().unwrap().update_settings(new_settings) {
            Ok(_) => { 
                status_label.set_text("Settings saved");
                reload_css();
                dialog_clone.close(); 
            },
            Err(e) => status_label.set_text(&format!("Error: {}", e)),
        }
    });

    dialog.present();
}

fn show_import_password_dialog<F>(
    parent: &ApplicationWindow,
    manager_rc: Arc<Mutex<CoreManager>>,
    status_label: Arc<Label>,
    refresh_list: F,
    import_path: std::path::PathBuf,
) where F: Fn() + 'static + Clone {
    let dialog = gtk::Window::builder()
        .title("Import")
        .modal(true)
        .transient_for(parent)
        .default_width(320)
        .default_height(200)
        .build();
    
    // Custom header
    let header = gtk::HeaderBar::new();
    header.set_show_title_buttons(false);
    header.add_css_class("custom-headerbar");
    
    let dialog_for_header = dialog.clone();
    let close_btn = gtk::Button::new();
    close_btn.add_css_class("traffic-btn");
    close_btn.add_css_class("traffic-close");
    close_btn.connect_clicked(move |_| dialog_for_header.close());
    
    let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    btn_box.set_margin_start(4);
    btn_box.append(&close_btn);
    header.pack_start(&btn_box);
    
    let header_title = Label::new(Some("Import"));
    header_title.add_css_class("headerbar-title");
    header.set_title_widget(Some(&header_title));
    
    dialog.set_titlebar(Some(&header));

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 12);
    vbox.set_margin_top(18);
    vbox.set_margin_bottom(18);
    vbox.set_margin_start(18);
    vbox.set_margin_end(18);

    let label = Label::new(Some("Enter password for import file:"));
    label.set_halign(gtk::Align::Start);

    let password_entry = gtk::PasswordEntry::new();
    password_entry.set_placeholder_text(Some("Password"));
    password_entry.set_show_peek_icon(true);

    let import_status = Rc::new(Label::new(None));
    import_status.set_halign(gtk::Align::Start);

    let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    button_box.set_halign(gtk::Align::End);

    let cancel_button = gtk::Button::with_label("Cancel");
    cancel_button.add_css_class("secondary-button");
    
    let import_button = gtk::Button::with_label("Import");
    import_button.add_css_class("action-button");

    button_box.append(&cancel_button);
    button_box.append(&import_button);

    vbox.append(&label);
    vbox.append(&password_entry);
    vbox.append(import_status.as_ref());
    vbox.append(&button_box);

    dialog.set_child(Some(&vbox));

    let dialog_clone = dialog.clone();
    cancel_button.connect_clicked(move |_| { dialog_clone.close(); });

    let dialog_clone = dialog.clone();
    let import_status_clone = import_status.clone();
    let password_entry_clone = password_entry.clone();
    
    import_button.connect_clicked(move |_| {
        let password = password_entry_clone.text().to_string();
        if password.is_empty() {
            import_status_clone.set_markup("<span foreground='#a06060'>Password required</span>");
            return;
        }

        let manager_clone = manager_rc.clone();
        let status_clone = status_label.clone();
        let path_clone = import_path.clone();
        let dialog_close = dialog_clone.clone();
        let import_status2 = import_status_clone.clone();
        let refresh = refresh_list.clone();
        
        import_status_clone.set_text("Importing...");
        
        let (sender, receiver) = async_channel::unbounded();
        let runtime = TOKIO_RUNTIME.get().unwrap();
        
        glib::spawn_future_local(async move {
            let _guard = runtime.enter();
            let result = tokio::task::spawn_blocking(move || {
                let pw = core::data::MasterPassword::from(password.as_str());
                manager_clone.lock().unwrap().import_encrypted(&path_clone, pw)
            }).await;
            let _ = sender.send(result).await;
        });
        
        glib::spawn_future_local(async move {
            if let Ok(result) = receiver.recv().await {
                match result {
                    Ok(Ok(_)) => { 
                        status_clone.set_text("Imported"); 
                        refresh(); 
                        dialog_close.close(); 
                    },
                    Ok(Err(e)) => import_status2.set_markup(&format!("<span foreground='#a06060'>{}</span>", e)),
                    Err(e) => import_status2.set_markup(&format!("<span foreground='#a06060'>{}</span>", e)),
                }
            }
        });
    });

    dialog.present();
    password_entry.grab_focus();
}
