---
name: Screenshot Feature
overview: Add a screenshot capture feature with a camera icon button in the tab bar and new screenshot settings (save path, file type, compression) in the Settings tab, persisted via the existing JSON cache system.
todos:
  - id: dep
    content: Add `image` crate to Cargo.toml
    status: completed
  - id: models
    content: Define ScreenshotFileType, ScreenshotCompression, and ScreenshotSettings in src/data/models.rs
    status: completed
  - id: appstate
    content: Add screenshot_settings field to AppState and load from cache on startup
    status: completed
  - id: camera-btn
    content: Add camera icon button in the right side of the tab bar in src/app.rs
    status: completed
  - id: capture-logic
    content: Implement screenshot capture via ViewportCommand::Screenshot and Event::Screenshot handler in src/app.rs
    status: completed
  - id: save-fn
    content: Implement save_screenshot function (ColorImage -> image crate encode -> disk) in src/app.rs
    status: completed
  - id: settings-ui
    content: Add screenshot settings section (save path, file type, compression) to src/ui/settings_view.rs
    status: completed
  - id: verify
    content: Verify compilation and fix any linter errors
    status: completed
isProject: false
---

# Screenshot Feature for Active Tab

## Context

The app is an egui/eframe desktop app (`[src/app.rs](src/app.rs)`). The top panel renders tab buttons on the left and a "Refresh Data" / spinner on the right using `Layout::right_to_left`. Settings live in `[src/ui/settings_view.rs](src/ui/settings_view.rs)` with persistence via `[src/data/cache.rs](src/data/cache.rs)` JSON helpers. No image-processing crate exists yet.

## New Dependency

Add the `image` crate to `[Cargo.toml](Cargo.toml)`:

```toml
image = "0.25"
```

This provides PNG/JPEG/TIFF encoding with compression control.

## A) Camera Icon Button in the Tab Bar

**File:** `[src/app.rs](src/app.rs)`

In the `right_to_left` layout section (line ~395), insert a camera button **before** the existing Refresh/spinner logic so it appears to the far right:

```rust
ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
    // Screenshot button (camera icon)
    if ui.button("📷").on_hover_text("Take screenshot").clicked() {
        self.capture_screenshot(ctx);
    }

    ui.separator();

    // existing Refresh / spinner logic ...
});
```

**Capture implementation** -- add a method `capture_screenshot` on `MktNoiseApp`:

```rust
fn capture_screenshot(&self, ctx: &egui::Context) {
    let settings = self.state.screenshot_settings.clone();
    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
    // Store settings snapshot so the callback can use them
}
```

egui's `ViewportCommand::Screenshot` requests a pixel buffer of the current frame. The result arrives on the **next** frame via `ctx.input(|i| i.events)` as `egui::Event::Screenshot`. We handle it in `update()`:

```rust
let events: Vec<egui::Event> = ctx.input(|i| i.events.clone());
for event in &events {
    if let egui::Event::Screenshot { image, .. } = event {
        save_screenshot(image, &self.state.screenshot_settings);
    }
}
```

`**save_screenshot` helper** (new free function or method):

1. Convert the `egui::ColorImage` pixel data to an `image::RgbaImage`.
2. Based on `ScreenshotSettings.file_type`, encode via the `image` crate:
  - **PNG** -- use `image::codecs::png::PngEncoder` with `CompressionType` mapped from the compression setting (None -> Default/Fast, Low -> Default, High -> Best).
  - **JPEG** -- use `image::codecs::jpeg::JpegEncoder::new_with_quality()` (None -> 100, Low -> 80, High -> 50).
  - **TIFF** -- use `image::codecs::tiff::TiffEncoder` (compression not directly supported by the crate for TIFF, so the setting is informational only; note this in a tooltip).
3. Build the file path: `{save_path}/{timestamp}.{ext}` with a `chrono::Local::now()` timestamp.
4. Write to disk and set `state.status_message` on success/failure.

## B) Screenshot Settings

### New Settings Struct

**File:** `[src/data/models.rs](src/data/models.rs)`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScreenshotFileType {
    Png,
    Jpeg,
    Tiff,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScreenshotCompression {
    None,
    Low,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotSettings {
    pub save_path: String,
    pub file_type: ScreenshotFileType,
    pub compression: ScreenshotCompression,
}

impl Default for ScreenshotSettings {
    fn default() -> Self {
        Self {
            save_path: "./screenshots".to_string(),
            file_type: ScreenshotFileType::Png,
            compression: ScreenshotCompression::None,
        }
    }
}
```

### Wire into AppState

**File:** `[src/app.rs](src/app.rs)`

Add `screenshot_settings: ScreenshotSettings` to `AppState` (line ~121) and initialize it in `Default` by attempting `load_json("screenshot_settings.json")` with a fallback to `ScreenshotSettings::default()`.

### Settings UI

**File:** `[src/ui/settings_view.rs](src/ui/settings_view.rs)`

Add a new section **above** the NN Training section:

```
render_screenshot_section(ui, state, &mut prev_visible);
```

The section renders:

- **Save Path** -- `ui.text_edit_singleline(&mut state.screenshot_settings.save_path)` with a label.
- **File Type** -- three `ui.selectable_value()` radio-style buttons for Png / Jpeg / Tiff.
- **Compression** -- three `ui.selectable_value()` radio-style buttons for None / Low / High.
- **Save Settings** button -- calls `save_json("screenshot_settings.json", &state.screenshot_settings)` and updates `status_message`.

### Persistence

Uses the existing `[src/data/cache.rs](src/data/cache.rs)` `save_json` / `load_json` helpers -- no changes needed there.

## Architecture Flow

```mermaid
sequenceDiagram
    participant User
    participant TabBar as "Tab Bar (app.rs)"
    participant Egui as "egui Context"
    participant Handler as "Event Handler (app.rs)"
    participant Disk as "Disk / image crate"

    User->>TabBar: Click camera icon
    TabBar->>Egui: ViewportCommand::Screenshot
    Egui->>Handler: Event::Screenshot with ColorImage
    Handler->>Disk: Encode (PNG/JPEG/TIFF) and save
    Disk-->>Handler: Success / Error
    Handler->>User: Status bar message
```



## Files Changed Summary


| File                      | Change                                                                                                                                 |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `Cargo.toml`              | Add `image = "0.25"`                                                                                                                   |
| `src/data/models.rs`      | Add `ScreenshotFileType`, `ScreenshotCompression`, `ScreenshotSettings`                                                                |
| `src/app.rs`              | Add `screenshot_settings` to `AppState`, camera button in tab bar, screenshot event handling, `capture_screenshot` + `save_screenshot` |
| `src/ui/settings_view.rs` | Add screenshot settings section with save path, file type, compression controls                                                        |


