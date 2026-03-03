//! Dashboard views

pub mod capture;
pub mod home;
pub mod overlay;
pub mod profiles;
pub mod screens;
pub mod settings;
pub mod vision;
pub mod zone_ocr;

pub use capture::render_capture_view;
pub use home::render_home_view;
pub use overlay::render_overlay_view;
pub use profiles::render_profiles_view;
pub use screens::render_screens_view;
pub use settings::render_settings_view;
pub use vision::render_vision_view;
