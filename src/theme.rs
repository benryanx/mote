use eframe::egui::{self, Color32};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub struct Theme {
    last: Instant,
    source: String,
    pub name: String,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            last: Instant::now() - Duration::from_secs(10),
            source: String::new(),
            name: "Mote dark".into(),
        }
    }
}
impl Theme {
    pub fn refresh(&mut self, ctx: &egui::Context) {
        if self.last.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.last = Instant::now();
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
            });
        let state = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state")
            });
        let current = state.join("omarchy/current/theme/colors.toml");
        let path = if current.exists() {
            current
        } else {
            config.join("omarchy/current/theme/colors.toml")
        };
        let source = std::fs::read_to_string(&path).unwrap_or_else(|_| {
            "background = '#171c2c'\nforeground = '#dce1eb'\naccent = '#a5c778'".into()
        });
        if self.source == source {
            return;
        }
        self.source = source.clone();
        let table = source.parse::<toml::Table>().unwrap_or_default();
        let color = |key: &str, fallback: Color32| {
            table
                .get(key)
                .and_then(|v| v.as_str())
                .and_then(|s| u32::from_str_radix(s.trim_start_matches('#'), 16).ok())
                .map(|v| Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
                .unwrap_or(fallback)
        };
        let bg = color("background", Color32::from_rgb(23, 28, 44));
        let fg = color("foreground", Color32::from_gray(220));
        let accent = color("accent", Color32::from_rgb(165, 199, 120));
        let light = (bg.r() as u32 + bg.g() as u32 + bg.b() as u32) > 420;
        let mut visuals = if light {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };
        visuals.panel_fill = bg;
        visuals.window_fill = bg;
        visuals.override_text_color = Some(fg);
        visuals.selection.bg_fill = accent.gamma_multiply(0.35);
        visuals.selection.stroke = egui::Stroke::new(1.0_f32, accent);
        visuals.widgets.active.bg_fill = accent.gamma_multiply(0.4);
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, accent);
        ctx.set_visuals(visuals);
        ctx.style_mut(|s| {
            s.spacing.item_spacing = egui::vec2(8.0, 7.0);
            s.spacing.button_padding = egui::vec2(10.0, 6.0);
        });
        self.name = std::fs::read_to_string(path.parent().unwrap().with_file_name("theme.name"))
            .map(|name| name.trim().to_string())
            .ok()
            .or_else(|| {
                std::fs::canonicalize(path.parent().unwrap())
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            })
            .unwrap_or("Mote dark".into());
    }
}
