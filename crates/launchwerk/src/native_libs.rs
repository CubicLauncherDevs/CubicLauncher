//! Completa `LD_LIBRARY_PATH` con las bibliotecas del sistema que un JRE
//! empaquetado no encuentra por sí solo.
//!
//! Los runtimes descargados (p. ej. Zulu de Adoptium) solo buscan en la caché
//! del cargador y en su propio RPATH. En NixOS no existe caché global y la
//! sesión gráfica expone las bibliotecas en `NIX_LD_LIBRARY_PATH` y
//! `/run/current-system/sw/lib`, así que AWT muere con
//! `libXtst.so.6: cannot open shared object file`. Lo mismo ocurre con
//! bibliotecas instaladas por el usuario pero expuestas fuera de esas rutas.
//!
//! Se añaden solo los directorios de las bibliotecas que el runtime necesita y
//! que el cargador actual no resuelve, y solo en Linux.

#[cfg(target_os = "linux")]
pub(crate) fn extend_ld_library_path(
    java: &std::path::Path,
    env: &mut std::collections::HashMap<String, String>,
) {
    let Some(native_dir) = java
        .parent()
        .and_then(|bin| bin.parent())
        .map(|home| home.join("lib").join("amd64"))
        .filter(|dir| dir.is_dir())
    else {
        return;
    };

    let mut dirs: Vec<String> = Vec::new();
    for library in linux::missing(&native_dir, env.get("LD_LIBRARY_PATH")) {
        match linux::locate(&library) {
            Some(dir) if !dirs.contains(&dir) => dirs.push(dir),
            Some(_) => {}
            None => log::warn!("El JRE necesita {library}, pero no está instalado en el sistema"),
        }
    }
    if dirs.is_empty() {
        return;
    }

    if let Some(existing) = env.get("LD_LIBRARY_PATH") {
        let inherited: Vec<String> = linux::paths(existing)
            .filter(|dir| !dirs.iter().any(|d| d == dir))
            .map(str::to_owned)
            .collect();
        dirs.extend(inherited);
    }
    log::info!("LD_LIBRARY_PATH ampliado con: {dirs:?}");
    env.insert("LD_LIBRARY_PATH".into(), dirs.join(":"));
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn extend_ld_library_path(
    _java: &std::path::Path,
    _env: &mut std::collections::HashMap<String, String>,
) {
}

#[cfg(target_os = "linux")]
mod linux {
    use std::path::{Path, PathBuf};

    const DT_NULL: u32 = 0;
    const DT_NEEDED: u32 = 1;
    const DT_STRTAB: u32 = 5;
    const PT_DYNAMIC: u32 = 2;

    /// Bibliotecas que piden los objetos de `native_dir` y que `runtime_path`
    /// (el valor actual de `LD_LIBRARY_PATH`) no cubre.
    pub(super) fn missing(native_dir: &Path, runtime_path: Option<&String>) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(native_dir) else {
            return Vec::new();
        };
        let mut libraries: Vec<String> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "so"))
            .filter_map(|path| std::fs::read(path).ok())
            .flat_map(|bytes| {
                Elf::parse(&bytes)
                    .map(|elf| elf.needed())
                    .unwrap_or_default()
            })
            .filter(|library| {
                !runtime_path.is_some_and(|value| {
                    paths(value).any(|dir| Path::new(dir).join(library).is_file())
                })
            })
            .collect();
        libraries.sort();
        libraries.dedup();
        libraries
    }

    /// Directorio que contiene `library`. Primero las rutas con que el sistema
    /// expone sus bibliotecas, después los paquetes del store de Nix y al
    /// final la caché de glibc (distribuciones clásicas).
    pub(super) fn locate(library: &str) -> Option<String> {
        if let Some(dir) = exported_dirs().find(|dir| Path::new(dir).join(library).is_file()) {
            return Some(dir);
        }
        if let Some(dir) = nix_package_dir(library) {
            return Some(dir);
        }
        cache_lookup(library)
    }

    /// Rutas con las que las sesiones de escritorio exponen bibliotecas.
    fn exported_dirs() -> impl Iterator<Item = String> {
        ["NIX_LD_LIBRARY_PATH", "LD_LIBRARY_PATH", "LIBRARY_PATH"]
            .into_iter()
            .flat_map(|var| std::env::var(var).ok())
            .flat_map(|value| {
                let dirs: Vec<String> = paths(&value).map(str::to_owned).collect();
                dirs
            })
            .chain(["/run/current-system/sw/lib".to_string()])
    }

    /// Busca el paquete de Nix que contiene `library` (`<hash>-<nombre>-<ver>`)
    /// bajo `<prefix>/<nombre>-<ver>/lib` o `<prefix>/lib/<nombre>-<ver>/lib`.
    /// Cubre bibliotecas instaladas pero ausentes de las rutas exportadas; es
    /// solo una búsqueda por nombre, sin invocar al cargador.
    fn nix_package_dir(library: &str) -> Option<String> {
        nix_package_dir_in(Path::new("/nix/store"), library)
            .or_else(|| nix_package_dir_in(Path::new("/nix/var/nix/profiles/default"), library))
    }

    pub(super) fn nix_package_dir_in(base: &Path, library: &str) -> Option<String> {
        // `libXtst.so.6` → `-libxtst-`.
        let needle = format!("-{}-", library.split(".so").next()?.to_ascii_lowercase());
        let mut packages: Vec<PathBuf> = std::fs::read_dir(base)
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.contains(&needle))
            })
            .collect();
        // La versión más reciente del store gana sobre el orden de entrada.
        packages.sort();
        packages.iter().rev().find_map(|package| {
            let direct = package.join("lib").join(library);
            if direct.is_file() {
                return Some(package.join("lib").display().to_string());
            }
            // Algunos envs anidan el paquete: `lib/<nombre>-<ver>/lib`.
            std::fs::read_dir(package.join("lib"))
                .ok()?
                .flatten()
                .find_map(|entry| {
                    let candidate = entry.path().join("lib").join(library);
                    candidate
                        .is_file()
                        .then(|| candidate.parent().map(|dir| dir.display().to_string()))
                        .flatten()
                })
        })
    }

    fn cache_lookup(library: &str) -> Option<String> {
        let ldconfig = ldconfig_path()?;
        let output = std::process::Command::new(ldconfig)
            .args(["-p", library])
            .output()
            .ok()?;
        let path = cache_entry(&String::from_utf8_lossy(&output.stdout), library)?;
        let real = std::fs::canonicalize(path).ok()?;
        Some(real.parent()?.display().to_string())
    }

    pub(super) fn paths(value: &str) -> impl Iterator<Item = &str> {
        value.split(':').filter(|dir| !dir.is_empty())
    }

    fn ldconfig_path() -> Option<String> {
        [
            "/run/current-system/sw/bin/ldconfig",
            "/sbin/ldconfig",
            "/usr/sbin/ldconfig",
        ]
        .into_iter()
        .find(|candidate| Path::new(candidate).is_file())
        .map(str::to_owned)
    }

    /// Línea de `ldconfig -p`: `libfoo.so.6 (libc6,x86-64) => /path/libfoo.so.6`.
    pub(super) fn cache_entry(cache: &str, library: &str) -> Option<PathBuf> {
        cache.lines().find_map(|line| {
            let (name, rest) = line.trim().split_once(" (")?;
            if name != library {
                return None;
            }
            rest.split_once("=> ")
                .map(|(_, path)| PathBuf::from(path.trim()))
        })
    }

    /// Cabecera y tabla dinámica de un ELF little-endian, suficiente para leer
    /// `DT_NEEDED`. Otros objetos (big-endian, 32 bits o corruptos) se ignoran.
    pub(super) struct Elf<'a> {
        bytes: &'a [u8],
        phoff: usize,
        phentsize: usize,
        phnum: usize,
    }

    impl<'a> Elf<'a> {
        pub(super) fn parse(bytes: &'a [u8]) -> Option<Self> {
            // Solo ELF64 little-endian: los JRE empaquetados de la plataforma.
            if bytes.get(..4)? != b"\x7fELF" || bytes.get(4)? != &2 || bytes.get(5)? != &1 {
                return None;
            }
            Some(Self {
                bytes,
                phoff: u64::from_le_bytes(bytes.get(32..40)?.try_into().ok()?) as usize,
                phentsize: u16::from_le_bytes(bytes.get(54..56)?.try_into().ok()?) as usize,
                phnum: u16::from_le_bytes(bytes.get(56..58)?.try_into().ok()?) as usize,
            })
        }

        pub(super) fn needed(&self) -> Vec<String> {
            let Some((dynamic, size)) = self.dynamic_segment() else {
                return Vec::new();
            };
            let strtab = self
                .dynamic_entries(dynamic, size)
                .find_map(|(tag, value)| (tag == DT_STRTAB).then_some(value));
            let Some(strtab) = strtab else {
                return Vec::new();
            };
            self.dynamic_entries(dynamic, size)
                .filter_map(|(tag, value)| {
                    (tag == DT_NEEDED)
                        .then(|| self.string(strtab + value))
                        .flatten()
                })
                .collect()
        }

        /// Offset y tamaño de `PT_DYNAMIC`.
        fn dynamic_segment(&self) -> Option<(usize, usize)> {
            (0..self.phnum).find_map(|index| {
                let base = self.phoff.checked_add(index.checked_mul(self.phentsize)?)?;
                let kind = u32::from_le_bytes(self.bytes.get(base..base + 4)?.try_into().ok()?);
                if kind != PT_DYNAMIC {
                    return None;
                }
                let offset =
                    u64::from_le_bytes(self.bytes.get(base + 8..base + 16)?.try_into().ok()?);
                let size =
                    u64::from_le_bytes(self.bytes.get(base + 32..base + 40)?.try_into().ok()?);
                Some((offset as usize, size as usize))
            })
        }

        fn dynamic_entries(
            &self,
            offset: usize,
            size: usize,
        ) -> impl Iterator<Item = (u32, u64)> + '_ {
            let bytes = self.bytes;
            (0..size / 16).map_while(move |entry| {
                let base = offset.checked_add(entry * 16)?;
                let tag = u32::from_le_bytes(bytes.get(base..base + 4)?.try_into().ok()?);
                let value = u64::from_le_bytes(bytes.get(base + 8..base + 16)?.try_into().ok()?);
                (tag != DT_NULL).then_some((tag, value))
            })
        }

        fn string(&self, offset: u64) -> Option<String> {
            let rest = self.bytes.get(usize::try_from(offset).ok()?..)?;
            let end = rest.iter().position(|byte| *byte == 0)?;
            String::from_utf8(rest[..end].to_vec()).ok()
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    /// JRE real de referencia: su `libawt_xawt.so` pide libX11, libXtst, libXi…
    fn jre_native_dir() -> Option<std::path::PathBuf> {
        let home = std::env::var_os("JAVA_HOME").or_else(|| std::env::var_os("CUBIC_TEST_JRE"))?;
        let native = std::path::Path::new(&home).join("lib").join("amd64");
        native.join("libawt_xawt.so").is_file().then_some(native)
    }

    #[test]
    fn reads_needed_libraries_from_a_real_elf() {
        let Some(dir) = jre_native_dir() else {
            return;
        };
        let bytes = std::fs::read(dir.join("libawt_xawt.so")).unwrap();
        let elf = linux::Elf::parse(&bytes).expect("libawt_xawt.so es un ELF64");
        let needed = elf.needed();
        for expected in ["libX11.so.6", "libXtst.so.6", "libjvm.so", "libc.so.6"] {
            assert!(needed.iter().any(|lib| lib == expected), "{needed:?}");
        }
        assert!(needed.iter().all(|lib| !lib.contains('/')));
    }

    #[test]
    fn rejects_non_elf_input() {
        assert!(linux::Elf::parse(b"#!/bin/sh\n").is_none());
        assert!(linux::Elf::parse(b"\x7fELF").is_none());
    }

    #[test]
    fn ldconfig_cache_entries_are_parsed_by_name() {
        let cache = "libXtst.so.6 (libc6,x86-64) => /nix/store/x-libxtst/lib/libXtst.so.6\n\
                     libXtst.so.4 (libc6,x86-64) => /old/libXtst.so.4";
        assert_eq!(
            linux::cache_entry(cache, "libXtst.so.6"),
            Some(std::path::PathBuf::from(
                "/nix/store/x-libxtst/lib/libXtst.so.6"
            ))
        );
        assert_eq!(linux::cache_entry(cache, "libXi.so.6"), None);
    }

    #[test]
    fn extend_resolves_the_real_jre_through_the_nix_store() {
        let Some(native) = jre_native_dir() else {
            return;
        };
        let java = native
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("bin")
            .join("java");
        let mut env = std::collections::HashMap::from([(
            "LD_LIBRARY_PATH".to_string(),
            "/run/current-system/sw/share/nix-ld/lib".to_string(),
        )]);
        extend_ld_library_path(&java, &mut env);
        let path = env.get("LD_LIBRARY_PATH").unwrap();
        for library in [
            "libXtst.so.6",
            "libX11.so.6",
            "libXi.so.6",
            "libXext.so.6",
            "libXrender.so.1",
        ] {
            assert!(
                path.split(':')
                    .any(|dir| std::path::Path::new(dir).join(library).is_file()),
                "{library} no está en {path}"
            );
        }
        assert_eq!(
            path.split(':').next_back(),
            Some("/run/current-system/sw/share/nix-ld/lib")
        );
    }

    #[cfg(unix)]
    #[test]
    fn nix_package_lookup_finds_the_library_by_package_name() {
        let temp = tempfile::tempdir().unwrap();
        let package = temp.path().join("abc-libxtst-1.2.6");
        std::fs::create_dir_all(package.join("lib")).unwrap();
        std::fs::write(package.join("lib").join("libXtst.so.6"), b"").unwrap();
        std::fs::create_dir_all(temp.path().join("zzz-libxi-1.8.2").join("lib")).unwrap();

        assert_eq!(
            linux::nix_package_dir_in(temp.path(), "libXtst.so.6"),
            Some(package.join("lib").display().to_string())
        );
        assert_eq!(linux::nix_package_dir_in(temp.path(), "libXi.so.6"), None);
        assert_eq!(linux::nix_package_dir_in(temp.path(), "libX11.so.6"), None);
    }
}
