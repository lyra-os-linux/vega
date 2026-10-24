// RPM builds inject the package release through VEGA_VERSION so the version
// shown by the application cannot drift from the package installed by the
// user. Direct Cargo builds fall back to the manifest version.
pub const APPLICATION_VERSION: &str = match option_env!("VEGA_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};
