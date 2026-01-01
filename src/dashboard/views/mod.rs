//! Dashboard views

pub mod home;
pub mod capture;
pub mod overlay;
pub mod vision;
pub mod screens;
pub mod profiles;
pub mod settings;
pub mod zone_ocr;

pub use home::render_home_view;
pub use capture::render_capture_view;
pub use overlay::render_overlay_view;
pub use vision::render_vision_view;
pub use screens::render_screens_view;
pub use profiles::render_profiles_view;
pub use settings::render_settings_view;
pub use zone_ocr::{render_zone_ocr_panel, draw_zone_overlays};
