//! Normalización de iconos de instancia.
//!
//! Un webview decodifica el archivo original completo aunque el icono se muestre
//! a 24 px: un PNG de 4096×4096 ocupa ~64 MB de RAM por instancia. Por eso acá
//! se normaliza una sola vez, al escribir, a un PNG de hasta 256 px por lado con
//! el hash en el nombre. Así el navegador solo decodifica un bitmap acotado y
//! puede reutilizarlo entre renders.

use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, RgbaImage};
use sha2::{Digest, Sha256};
use std::fmt::Write as FmtWrite;
use std::io::{BufRead, Cursor, Seek, Write as IoWrite};
use std::path::{Path, PathBuf};

/// Lado máximo del icono final. Coincide con el presupuesto que ya usa
/// `addon_manager` para los iconos de mods.
pub(crate) const ICON_DIMENSION: u32 = 256;
/// Tope del archivo de entrada antes de decodificar.
pub(crate) const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
/// Tope de dimensiones declaradas por el archivo. Un bitmap mayor se rechaza
/// antes de reservar memoria.
const MAX_SOURCE_DIMENSION: u32 = 8192;
/// Tope de memoria que puede reservar el decodificador.
const MAX_DECODE_ALLOC: u64 = 128 * 1024 * 1024;
const HASH_BYTES: usize = 8;

pub(crate) struct NormalizedIcon {
    pub bytes: Vec<u8>,
    pub hash: String,
}

impl NormalizedIcon {
    pub(crate) fn file_name(&self) -> String {
        format!("icon_{}.png", self.hash)
    }
}

/// Lee y normaliza un archivo de icono. Aplica el tope de tamaño antes de
/// decodificar para no cargar archivos arbitrariamente grandes.
pub(crate) fn normalize_icon_file(path: &Path) -> Result<NormalizedIcon, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("No se pudo abrir la imagen: {e}"))?;
    let size = file
        .metadata()
        .map_err(|e| format!("No se pudo leer la imagen: {e}"))?
        .len();
    if size > MAX_SOURCE_BYTES {
        return Err("La imagen supera el límite de 64 MB".into());
    }
    // Se decodifica directamente desde el archivo: no hace falta mantener una
    // copia completa de los bytes comprimidos en memoria.
    normalize_icon_reader(std::io::BufReader::new(file))
}

/// Detecta el formato real por contenido (no por extensión), decodifica con
/// límites, corrige la orientación EXIF y reduce con Lanczos conservando la
/// relación de aspecto. Nunca amplía una imagen chica ni recorta.
#[cfg(test)]
pub(crate) fn normalize_icon_bytes(bytes: &[u8]) -> Result<NormalizedIcon, String> {
    if bytes.is_empty() {
        return Err("El archivo está vacío".into());
    }
    if bytes.len() as u64 > MAX_SOURCE_BYTES {
        return Err("La imagen supera el límite de 64 MB".into());
    }
    normalize_icon_reader(Cursor::new(bytes))
}

fn normalize_icon_reader<R: BufRead + Seek>(reader: R) -> Result<NormalizedIcon, String> {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_SOURCE_DIMENSION);
    limits.max_image_height = Some(MAX_SOURCE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);

    let mut reader = ImageReader::new(reader)
        .with_guessed_format()
        .map_err(|e| format!("No se pudo detectar el formato: {e}"))?;
    if reader.format().is_none() {
        return Err("El archivo no es una imagen compatible".into());
    }
    reader.limits(limits);

    let mut decoder = reader
        .into_decoder()
        .map_err(|e| format!("No se pudo decodificar la imagen: {e}"))?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder)
        .map_err(|e| format!("No se pudo decodificar la imagen: {e}"))?;
    image.apply_orientation(orientation);

    let fitted = fit_without_upscale(&image, ICON_DIMENSION);
    let mut out = Vec::new();
    fitted
        .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
        .map_err(|e| format!("No se pudo codificar el icono: {e}"))?;

    Ok(NormalizedIcon {
        hash: short_hash(&out),
        bytes: out,
    })
}

fn fit_without_upscale(image: &DynamicImage, max: u32) -> DynamicImage {
    let (width, height) = (image.width(), image.height());
    if width <= max && height <= max {
        return DynamicImage::ImageRgba8(image.to_rgba8());
    }

    let (target_w, target_h) = fit_dimensions(width, height, max);
    if image.color().has_alpha() {
        let rgba = image.to_rgba8();
        DynamicImage::ImageRgba8(resize_premultiplied(rgba, target_w, target_h))
    } else {
        // Sin alfa no hace falta duplicar el bitmap completo: se reduce en su
        // formato original.
        image.resize_exact(target_w, target_h, FilterType::Lanczos3)
    }
}

fn fit_dimensions(width: u32, height: u32, max: u32) -> (u32, u32) {
    let max = u64::from(max);
    if width >= height {
        (
            max as u32,
            ((u64::from(height) * max) / u64::from(width)).max(1) as u32,
        )
    } else {
        (
            ((u64::from(width) * max) / u64::from(height)).max(1) as u32,
            max as u32,
        )
    }
}

/// Lanczos sobre RGBA con alfa recto deja halos oscuros en los bordes
/// transparentes. Premultiplicar antes de reducir y des-premultiplicar después
/// conserva el color de los píxeles semitransparentes.
fn resize_premultiplied(mut source: RgbaImage, width: u32, height: u32) -> RgbaImage {
    for pixel in source.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for channel in 0..3 {
            pixel[channel] = ((u16::from(pixel[channel]) * alpha + 127) / 255) as u8;
        }
    }

    let mut resized = image::imageops::resize(&source, width, height, FilterType::Lanczos3);
    for pixel in resized.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        if alpha == 0 {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
            continue;
        }
        for channel in 0..3 {
            pixel[channel] = ((u16::from(pixel[channel]) * 255 + alpha / 2) / alpha).min(255) as u8;
        }
    }
    resized
}

fn short_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hash = String::with_capacity(HASH_BYTES * 2);
    for byte in digest.iter().take(HASH_BYTES) {
        let _ = write!(hash, "{byte:02x}");
    }
    hash
}

/// Escribe el icono con nombre content-addressed. Si ya existe el mismo hash no
/// reescribe nada: el webview reutiliza la imagen decodificada de esa URL.
pub(crate) fn store_normalized_icon(
    icons_dir: &Path,
    icon: &NormalizedIcon,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(icons_dir)
        .map_err(|e| format!("No se pudo crear el directorio de iconos: {e}"))?;

    let destination = icons_dir.join(icon.file_name());
    if destination.is_file() {
        return Ok(destination);
    }

    let mut temporary = tempfile::NamedTempFile::new_in(icons_dir)
        .map_err(|e| format!("No se pudo preparar el icono: {e}"))?;
    temporary
        .write_all(&icon.bytes)
        .map_err(|e| format!("No se pudo escribir el icono: {e}"))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|e| format!("No se pudo sincronizar el icono: {e}"))?;

    match temporary.persist(&destination) {
        Ok(_) => Ok(destination),
        // Otra escritura con el mismo hash ganó la carrera: el archivo ya es
        // válido porque el nombre deriva del contenido.
        Err(error) if destination.is_file() => {
            drop(error);
            Ok(destination)
        }
        Err(error) => Err(format!("No se pudo guardar el icono: {}", error.error)),
    }
}

/// Borra iconos anteriores de la instancia. Solo toca los nombres que genera
/// esta app (`icon_*` y los `custom_*` legacy), nunca archivos ajenos.
pub(crate) fn remove_icon_files(icons_dir: &Path, keep: Option<&Path>) {
    let Ok(entries) = std::fs::read_dir(icons_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if keep.is_some_and(|keep| keep == path.as_path()) {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !(name.starts_with("icon_") || name.starts_with("custom_")) {
            continue;
        }
        if path.is_file() {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        png_bytes_with_color(width, height, [200, 40, 10, 255])
    }

    fn png_bytes_with_color(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
        let image = RgbaImage::from_pixel(width, height, Rgba(color));
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn downscales_large_icons_to_the_max_side() {
        let normalized = normalize_icon_bytes(&png_bytes(1024, 1024)).unwrap();
        let image = image::load_from_memory(&normalized.bytes).unwrap();

        assert_eq!(
            (image.width(), image.height()),
            (ICON_DIMENSION, ICON_DIMENSION)
        );
        assert_eq!(normalized.hash.len(), HASH_BYTES * 2);
    }

    #[test]
    fn small_icons_are_not_upscaled() {
        let normalized = normalize_icon_bytes(&png_bytes(32, 32)).unwrap();
        let image = image::load_from_memory(&normalized.bytes).unwrap();

        assert_eq!((image.width(), image.height()), (32, 32));
    }

    #[test]
    fn non_square_icons_keep_their_aspect_ratio() {
        let normalized = normalize_icon_bytes(&png_bytes(800, 200)).unwrap();
        let image = image::load_from_memory(&normalized.bytes).unwrap();

        assert_eq!((image.width(), image.height()), (ICON_DIMENSION, 64));
    }

    #[test]
    fn jpeg_is_detected_by_content_and_saved_as_png() {
        let source = image::RgbImage::from_pixel(512, 512, image::Rgb([20, 30, 40]));
        let mut jpeg = Vec::new();
        DynamicImage::ImageRgb8(source)
            .write_to(&mut Cursor::new(&mut jpeg), ImageFormat::Jpeg)
            .unwrap();

        let normalized = normalize_icon_bytes(&jpeg).unwrap();
        assert_eq!(&normalized.bytes[..8], b"\x89PNG\r\n\x1a\n");
        let image = image::load_from_memory(&normalized.bytes).unwrap();
        assert_eq!(
            (image.width(), image.height()),
            (ICON_DIMENSION, ICON_DIMENSION)
        );
    }

    #[test]
    fn corrupt_input_is_rejected() {
        assert!(normalize_icon_bytes(b"not an image").is_err());
        assert!(normalize_icon_bytes(&[]).is_err());
    }

    #[test]
    fn the_same_content_produces_the_same_name() {
        let bytes = png_bytes(512, 512);
        assert_eq!(
            normalize_icon_bytes(&bytes).unwrap().hash,
            normalize_icon_bytes(&bytes).unwrap().hash
        );
        assert_ne!(
            normalize_icon_bytes(&bytes).unwrap().hash,
            normalize_icon_bytes(&png_bytes_with_color(256, 256, [10, 200, 30, 255]))
                .unwrap()
                .hash
        );
    }

    #[test]
    fn storing_and_cleaning_keeps_only_the_current_icon() {
        let dir = tempfile::tempdir().unwrap();
        let icons = dir.path().join("icons");
        std::fs::create_dir(&icons).unwrap();
        std::fs::write(icons.join("custom_1.png"), b"old").unwrap();
        std::fs::write(icons.join("icon_aaaaaaaaaaaaaaaa.png"), b"old").unwrap();
        std::fs::write(icons.join("unrelated.txt"), b"keep").unwrap();

        let normalized = normalize_icon_bytes(&png_bytes(512, 512)).unwrap();
        let stored = store_normalized_icon(&icons, &normalized).unwrap();
        assert!(stored.is_file());
        assert_eq!(store_normalized_icon(&icons, &normalized).unwrap(), stored);

        remove_icon_files(&icons, Some(&stored));
        assert!(stored.is_file());
        assert!(!icons.join("custom_1.png").exists());
        assert!(!icons.join("icon_aaaaaaaaaaaaaaaa.png").exists());
        assert!(icons.join("unrelated.txt").exists());
    }
}
