//! Providers de importación para launchers de terceros.
//!
//! Cada submódulo implementa `InstanceImporter` para un formato concreto.

mod multimc;

pub use multimc::MultimcProvider;
