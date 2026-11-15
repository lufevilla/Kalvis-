# Flutter-Rust Bridge Integration Design

## Overview
Create a shared Rust library (`space-health-lib`) that exposes tracking data functions via `flutter_rust_bridge`. The Flutter app will call these functions directly via FFI, eliminating HTTP dependency. The existing backend server (`back-end`) continues running independently with its 5-second monitoring loop.

## Architecture

```
NASA SPACE APPS/
├── back-end/                    # Binary: HTTP server + monitoring (unchanged)
│   ├── Cargo.toml               # Depends on space-health-lib
│   └── src/main.rs              # Starts server + monitoring
├── space-health-lib/            # NEW: Shared library crate
│   ├── Cargo.toml               # cdylib + rlib, flutter_rust_bridge
│   ├── flutter_rust_bridge.yaml # Codegen config
│   └── src/
│       ├── lib.rs               # Exports + frb setup
│       ├── tracking.rs          # get_tracking_data(), init_db()
│       ├── database.rs          # SQLite pool + queries
│       └── models.rs            # Frb-serializable structs
├── front_end/                   # Flutter app
│   ├── flutter_rust_bridge.yaml # Codegen config
│   └── lib/
│       └── bridge_generated.dart # Auto-generated bindings
```

## Data Flow

```
Flutter UI (TrackingScreen)
    ↓
bridge_generated.dart (Dart)
    ↓ FFI
space-health-lib (Rust cdylib)
    ↓ sqlx
SQLite Database (file)
    ↑
Tracking data returned as struct
```

## Rust Library API

### `space-health-lib/src/models.rs`
```rust
#[frb]
pub struct TrackingVital {
    pub title: String,
    pub value: String,
    pub unit: String,
    pub status: String,      // "normal" | "attention" | "alert"
    pub status_label: String,
    pub range_min: Option<f64>,
    pub range_max: Option<f64>,
    pub normal_min: Option<f64>,
    pub normal_max: Option<f64>,
    pub description: Option<String>,
    pub action_text: Option<String>,
}

#[frb]
pub struct TrackingEnvironment {
    pub title: String,
    pub value: String,
    pub unit: String,
    pub status: String,
    pub status_label: String,
    pub description: Option<String>,
    pub action_text: Option<String>,
}

#[frb]
pub struct TrackingData {
    pub crew_vitals: Vec<TrackingVital>,
    pub environment_metrics: Vec<TrackingEnvironment>,
    pub last_update: String,
    pub astronaut_id: String,
    pub mission_name: Option<String>,
    pub current_module: Option<String>,
}
```

### `space-health-lib/src/tracking.rs`
```rust
use crate::models::{TrackingData, TrackingVital, TrackingEnvironment};

#[frb(sync)]
pub fn init_database(db_path: String) -> Result<(), String> { ... }

#[frb(sync)]
pub fn get_tracking_data(astronaut_id: String) -> Result<TrackingData, String> { ... }
```

## Implementation Details

### Database
- SQLite file path passed from Flutter via `init_database()`
- Uses existing `sqlx` queries from `back-end` (measurement_repository)
- Single connection pool, initialized once

### Backend Server (`back-end`)
- Continues unchanged: HTTP server on :3000 + monitoring loop every 5s
- Adds `space-health-lib` as dependency
- Shares same database file

### Flutter
- `flutter_rust_bridge_codegen integrate --rust-crate-dir ../space-health-lib`
- Generated bindings in `lib/bridge_generated.dart`
- `TrackingScreen` calls `SpaceHealthLib().getTrackingData()` directly
- Falls back to mocks on error

## Platform Support
- **Linux**: cdylib (`.so`) - primary target
- **Android**: cdylib (`.so`) via NDK - future
- **iOS**: staticlib (`.a`) - future
- **Web**: NOT supported (requires WASM, different approach)

## File Changes

### New Files
- `space-health-lib/Cargo.toml`
- `space-health-lib/flutter_rust_bridge.yaml`
- `space-health-lib/src/lib.rs`
- `space-health-lib/src/models.rs`
- `space-health-lib/src/database.rs`
- `space-health-lib/src/tracking.rs`

### Modified Files
- `back-end/Cargo.toml` - add `space-health-lib` dependency
- `front_end/pubspec.yaml` - add `flutter_rust_bridge` dependency
- `front_end/flutter_rust_bridge.yaml` - codegen config
- `front_end/lib/core/services/tracking_api_service.dart` - replace HTTP with bridge
- `front_end/lib/screens/tracking/tracking_screen.dart` - use bridge

## Success Criteria
1. `flutter build linux` compiles Rust library automatically
2. `TrackingScreen` loads data from Rust library via FFI
3. Backend server still runs monitoring independently
4. Same SQLite database file shared between both