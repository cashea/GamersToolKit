//! Dashboard views

pub mod home;
pub mod capture;
pub mod overlay;
pub mod profiles;
pub mod settings;

pub use home::render_home_view;
pub use capture::render_capture_view;
pub use overlay::render_overlay_view;
pub use profiles::render_profiles_view;
pub use settings::render_settings_view;
