use serde::{Deserialize, Serialize};

/// Represents a package, and contains all the metadata assoicated with it.
///
/// [`sqlx::FromRow`][sqlx::FromRow] is derived, so this should match the database schema
/// as specified in `scripts/init_db.py`.
#[derive(Deserialize, Default, Serialize, Clone, Debug, PartialEq, Eq, sqlx::FromRow)]
pub struct Package {
    /// The package's name, ie "gcc"
    /// This is the primary key
    pub pkgname: String,
    /// The game/app's full name/title, ie "The GNU Compiler Collection, Version 4.3"
    pub fullname: String,
    /// A short description of the package
    #[sqlx(default)]
    pub description: Option<String>,
    /// A URL pointing to an image for the package
    pub image_url: Option<String>,
    /// The relative path of the executable within the tarball
    pub executable_path: Option<String>,
    /// The package's CRC checksum
    pub crc: u32,
    /// Does the package have an install script that needs running?
    pub has_installer: bool,
    /// Does the package want to be added to path on the machine it was installed on?
    pub add_to_path: bool,
}
