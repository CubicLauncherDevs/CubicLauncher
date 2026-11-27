//! Materializa los índices de assets legacy que el juego lee fuera del
//! almacén `objects` compartido.
//!
//! - Índices `virtual` (1.6–1.7.2): el juego lee `<assets>/virtual/<id>/<ruta>`,
//!   por lo que los objetos del índice se descomprimen ahí.
//! - Pre-1.6: el juego lee `<gameDir>/resources/<ruta>`, así que la instancia
//!   recibe una copia de los assets.

use std::io;
use std::path::{Path, PathBuf};

use sha1::{Digest, Sha1};
use zellkern::VersionManifest;
use zellkern::path_security::ConfinedDir;

const MARKER_PREFIX: &str = ".cubic-legacy-";
const MARKER_SUFFIX: &str = ".sha1";

/// Prepara los assets legacy de una instancia.
///
/// Devuelve el directorio que el juego debe usar como assets, o `None` cuando
/// el índice es moderno y basta con `<shared>/assets`.
pub(crate) fn prepare(
    manifest: &VersionManifest,
    shared_dir: &Path,
    instance_dir: &Path,
) -> Result<Option<PathBuf>, String> {
    let (resolved, _) = VersionManifest::resolve_chain(manifest, shared_dir)
        .map_err(|e| format!("No se pudo resolver la versión: {e}"))?;
    let index_id = resolved
        .asset_index
        .as_ref()
        .map(|index| index.id.clone())
        .unwrap_or_default();
    if index_id.is_empty() {
        return Ok(None);
    }

    let assets_dir = shared_dir.join("assets");
    let index_path = assets_dir.join("indexes").join(format!("{index_id}.json"));
    let index_bytes = match std::fs::read(&index_path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(format!(
                "Falta el índice de assets de Minecraft: {}",
                index_path.display()
            ));
        }
        Err(e) => return Err(format!("No se pudo leer {:?}: {e}", index_path)),
    };
    let index: aqua::VersionAssets = serde_json::from_slice(&index_bytes)
        .map_err(|e| format!("Índice de assets inválido {:?}: {e}", index_path))?;

    let is_pre_1_6 = zellkern::is_pre_1_6(&resolved.id_raw);
    // Indices from 1.6+ carry no legacy flag and the game reads `objects`
    // directly; only versions before 1.6 keep their assets elsewhere.
    if index.r#virtual.is_none() && index.map_to_resources.is_none() && !is_pre_1_6 {
        return Ok(None);
    }
    let objects = assets_dir.join("objects");
    let state = if index.map_to_resources == Some(true) || (index.r#virtual.is_none() && is_pre_1_6)
    {
        State::in_instance(instance_dir, &assets_dir, objects, &index, &index_bytes)
    } else {
        State::in_virtual_root(
            &assets_dir.join("virtual").join(&index_id),
            objects,
            &index,
            &index_bytes,
        )
    };

    if !state.is_current()? {
        state.materialize()?;
    }
    Ok(Some(state.game_assets))
}

struct State {
    assets: aqua::VersionAssets,
    game_assets: PathBuf,
    objects: PathBuf,
    index_sha1: String,
    /// Directorio del marcador; `None` para escribir junto a los assets.
    marker_dir: Option<PathBuf>,
}

impl State {
    fn in_instance(
        instance_dir: &Path,
        marker_dir: &Path,
        objects: PathBuf,
        index: &aqua::VersionAssets,
        bytes: &[u8],
    ) -> Self {
        Self::new(
            instance_dir.join("resources"),
            objects,
            index,
            bytes,
            Some(marker_dir.to_path_buf()),
        )
    }

    fn in_virtual_root(
        root: &Path,
        objects: PathBuf,
        index: &aqua::VersionAssets,
        bytes: &[u8],
    ) -> Self {
        // `<assets>/virtual` queda fuera de la raíz que abre el juego.
        let marker_dir = root.parent().map(Path::to_path_buf);
        Self::new(root.to_path_buf(), objects, index, bytes, marker_dir)
    }

    fn new(
        game_assets: PathBuf,
        objects: PathBuf,
        index: &aqua::VersionAssets,
        bytes: &[u8],
        marker_dir: Option<PathBuf>,
    ) -> Self {
        let index_sha1: String = Sha1::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Self {
            assets: index.clone(),
            game_assets,
            objects,
            index_sha1,
            marker_dir,
        }
    }

    /// Un marcador con el SHA-1 del índice registra que ya se descomprimió,
    /// para no recorrer miles de archivos en cada arranque.
    ///
    /// Vive en `<assets>`, nunca en `resources`: el juego escanea esa carpeta
    /// como resource pack y registra en el log los archivos que no entiende.
    fn marker_path(&self) -> PathBuf {
        let marker = format!("{MARKER_PREFIX}{}{MARKER_SUFFIX}", self.index_sha1);
        match self.marker_dir {
            Some(ref dir) => dir.join(marker),
            None => self.game_assets.join(marker),
        }
    }

    fn is_current(&self) -> Result<bool, String> {
        let marker = self.marker_path();
        match std::fs::symlink_metadata(&marker) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(format!("No se pudo leer {:?}: {e}", marker)),
            Ok(_) => std::fs::read(&marker)
                .map(|content| content == self.index_sha1.as_bytes())
                .map_err(|e| format!("No se pudo leer {:?}: {e}", marker)),
        }
    }

    fn materialize(&self) -> Result<(), String> {
        let game_assets = ConfinedDir::open_or_create(&self.game_assets)
            .map_err(|e| format!("No se pudo abrir {:?}: {e}", self.game_assets))?;
        let objects = ConfinedDir::open(&self.objects)
            .map_err(|e| format!("No se pudieron abrir los assets compartidos: {e}"))?;

        for (path, asset) in &self.assets.objects {
            if game_assets.file_exists(Path::new(path)).unwrap_or(false) {
                continue;
            }
            let source = object_path(&asset.hash);
            let bytes = objects
                .read(Path::new(&source))
                .map_err(|e| format!("Falta el asset compartido {source}: {e}"))?;
            game_assets
                .copy_from(Path::new(path), &mut bytes.as_slice())
                .map_err(|e| format!("No se pudo escribir el asset {path}: {e}"))?;
        }

        // Solo se escribe al terminar: un fallo a medias se reintenta entero.
        std::fs::write(self.marker_path(), &self.index_sha1)
            .map_err(|e| format!("No se pudo guardar el marcador de assets legacy: {e}"))
    }
}

fn object_path(hash: &str) -> String {
    format!("{}/{hash}", &hash[..2.min(hash.len())])
}

#[cfg(test)]
#[path = "../tests/services/legacyfix.rs"]
mod tests;
