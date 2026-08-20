//! Providers de importación para launchers de terceros.
//!
//! Cada submódulo implementa `InstanceImporter` para un formato concreto.

mod cubic;
mod multimc;

pub use cubic::CubicProvider;
pub use multimc::MultimcProvider;
