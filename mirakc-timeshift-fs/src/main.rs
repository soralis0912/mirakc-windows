#[cfg(unix)]
mod app;
#[cfg(unix)]
mod filesystem;

// `fuser::mount()` was deprecatged, but `fuser::mnt::mount_options::parse_options_from_args()`
// is still private.  So, there is no way to parse mount options specified in the command line...
// We copied mnt/mount_options.rs from cberner/fuser.
#[cfg(unix)]
mod mount_options;

#[cfg(unix)]
fn main() -> Result<(), mirakc_core::error::Error> {
    app::main()
}

// FUSE is not available on Windows.
#[cfg(not(unix))]
fn main() {
    eprintln!("mirakc-timeshift-fs is not supported on this platform");
    std::process::exit(1);
}
