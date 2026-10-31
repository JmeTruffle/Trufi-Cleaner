//! Rule packs compiled into the binary. Add a new file under `rules/` and
//! list it here.

macro_rules! packs {
    ($($path:literal),* $(,)?) => {
        &[$(($path, include_str!(concat!("../../../../rules/", $path)))),*]
    };
}

static PACKS: &[(&str, &str)] = packs![
    "windows/system.toml",
    "windows/browsers.toml",
    "windows/apps.toml",
    "windows/games.toml",
    "windows/developer.toml",
    "macos/system.toml",
    "macos/browsers.toml",
    "macos/apps.toml",
    "macos/games.toml",
    "macos/developer.toml",
];

pub fn builtin_packs() -> &'static [(&'static str, &'static str)] {
    PACKS
}
