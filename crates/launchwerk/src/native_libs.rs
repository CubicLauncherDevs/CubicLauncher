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
    let Some(home) = java.parent().and_then(|bin| bin.parent()) else {
        return;
    };
    let Some(native_dir) = linux::native_dir(home) else {
        return;
    };
    let Some(expected) = linux::expected_id(&native_dir) else {
        return;
    };

    let mut dirs: Vec<String> = Vec::new();
    for library in linux::missing(&native_dir, env.get("LD_LIBRARY_PATH")) {
        match linux::locate(&library, expected) {
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

    /// Clase (`EI_CLASS`) y máquina (`e_machine`) de un objeto ELF.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(super) struct ElfId {
        pub(super) class: u8,
        pub(super) machine: u16,
    }

    /// Directorio de bibliotecas nativas del JRE (`lib/amd64`, `lib/aarch64`…).
    pub(super) fn native_dir(home: &Path) -> Option<PathBuf> {
        let lib = home.join("lib");
        let arch = match std::env::consts::ARCH {
            "x86_64" => Some("amd64"),
            "x86" => Some("i386"),
            "aarch64" => Some("aarch64"),
            "arm" => Some("arm"),
            _ => None,
        };
        if let Some(arch) = arch {
            let candidate = lib.join(arch);
            if candidate.is_dir() {
                return Some(candidate);
            }
        }
        std::fs::read_dir(&lib)
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .find(|dir| dir.is_dir() && contains_native_library(dir))
    }

    fn contains_native_library(dir: &Path) -> bool {
        std::fs::read_dir(dir).ok().is_some_and(|entries| {
            entries.flatten().any(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "so")
            })
        })
    }

    /// Arquitectura que deben tener las bibliotecas del sistema para que el
    /// JRE las pueda cargar. Se lee del propio runtime y, si no es posible,
    /// se asume la del host.
    pub(super) fn expected_id(native_dir: &Path) -> Option<ElfId> {
        ["libawt_xawt.so", "libjava.so", "libjvm.so"]
            .into_iter()
            .find_map(|name| file_id(&native_dir.join(name)))
            .or_else(|| {
                std::fs::read_dir(native_dir)
                    .ok()?
                    .flatten()
                    .map(|entry| entry.path())
                    .find_map(|path| file_id(&path))
            })
            .or_else(host_id)
    }

    fn host_id() -> Option<ElfId> {
        let machine = match std::env::consts::ARCH {
            "x86_64" => 62,
            "x86" => 3,
            "aarch64" => 183,
            "arm" => 40,
            "riscv64" => 243,
            "powerpc64" => 21,
            "s390x" => 22,
            _ => return None,
        };
        Some(ElfId { class: 2, machine })
    }

    /// Identidad de la cabecera ELF, sin importar si es de 32 o 64 bits.
    pub(super) fn elf_id(bytes: &[u8]) -> Option<ElfId> {
        if bytes.get(..4)? != b"\x7fELF" {
            return None;
        }
        let class = *bytes.get(4)?;
        if class != 1 && class != 2 {
            return None;
        }
        let machine = match *bytes.get(5)? {
            1 => u16::from_le_bytes(bytes.get(18..20)?.try_into().ok()?),
            2 => u16::from_be_bytes(bytes.get(18..20)?.try_into().ok()?),
            _ => return None,
        };
        Some(ElfId { class, machine })
    }

    pub(super) fn file_id(path: &Path) -> Option<ElfId> {
        use std::io::Read;

        let mut header = [0u8; 20];
        std::fs::File::open(path)
            .ok()?
            .read_exact(&mut header)
            .ok()?;
        elf_id(&header)
    }

    fn compatible(path: &Path, expected: ElfId) -> bool {
        file_id(path) == Some(expected)
    }

    /// Directorio que contiene `library`. Primero las rutas con que el sistema
    /// expone sus bibliotecas, después los paquetes del store de Nix y al
    /// final la caché de glibc (distribuciones clásicas). Solo se aceptan
    /// objetos con la misma arquitectura que el JRE.
    pub(super) fn locate(library: &str, expected: ElfId) -> Option<String> {
        if let Some(dir) =
            exported_dirs().find(|dir| compatible(&Path::new(dir).join(library), expected))
        {
            return Some(dir);
        }
        if let Some(dir) = nix_package_dir(library, expected) {
            return Some(dir);
        }
        cache_lookup(library, expected)
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
    fn nix_package_dir(library: &str, expected: ElfId) -> Option<String> {
        nix_package_dir_in(Path::new("/nix/store"), library, expected).or_else(|| {
            nix_package_dir_in(
                Path::new("/nix/var/nix/profiles/default"),
                library,
                expected,
            )
        })
    }

    pub(super) fn nix_package_dir_in(
        base: &Path,
        library: &str,
        expected: ElfId,
    ) -> Option<String> {
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
        packages.sort_by(|left, right| {
            version_key(left)
                .cmp(&version_key(right))
                .then_with(|| left.cmp(right))
        });
        packages
            .iter()
            .rev()
            .find_map(|package| package_dir(package, library, expected))
    }

    fn package_dir(package: &Path, library: &str, expected: ElfId) -> Option<String> {
        let direct = package.join("lib").join(library);
        if compatible(&direct, expected) {
            return Some(package.join("lib").display().to_string());
        }
        // Algunos envs anidan el paquete: `lib/<nombre>-<ver>/lib`.
        std::fs::read_dir(package.join("lib"))
            .ok()?
            .flatten()
            .find_map(|entry| {
                let candidate = entry.path().join("lib").join(library);
                compatible(&candidate, expected)
                    .then(|| candidate.parent().map(|dir| dir.display().to_string()))
                    .flatten()
            })
    }

    /// Componente numérica final del nombre del paquete (`...-libxtst-1.2.6`).
    fn version_key(package: &Path) -> Vec<u64> {
        package
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| {
                name.split('-')
                    .rev()
                    .find(|part| part.chars().next().is_some_and(|c| c.is_ascii_digit()))
            })
            .map(|version| {
                version
                    .split('.')
                    .filter_map(|part| part.parse().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn cache_lookup(library: &str, expected: ElfId) -> Option<String> {
        let ldconfig = ldconfig_path()?;
        let output = std::process::Command::new(ldconfig)
            .arg("-p")
            .output()
            .ok()?;
        let cache = String::from_utf8_lossy(&output.stdout);
        let path = cache_entries(&cache, library).find(|path| file_id(path) == Some(expected))?;
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
    pub(super) fn cache_entries<'a>(
        cache: &'a str,
        library: &'a str,
    ) -> impl Iterator<Item = PathBuf> + 'a {
        cache.lines().filter_map(move |line| {
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
                     libXtst.so.6 (libc6) => /lib32/libXtst.so.6\n\
                     libXtst.so.4 (libc6,x86-64) => /old/libXtst.so.4";
        assert_eq!(
            linux::cache_entries(cache, "libXtst.so.6").collect::<Vec<_>>(),
            vec![
                std::path::PathBuf::from("/nix/store/x-libxtst/lib/libXtst.so.6"),
                std::path::PathBuf::from("/lib32/libXtst.so.6"),
            ]
        );
        assert_eq!(linux::cache_entries(cache, "libXi.so.6").count(), 0);
    }

    #[test]
    fn elf_identity_distinguishes_32_and_64_bits() {
        let mut header = [0u8; 20];
        header[..4].copy_from_slice(b"\x7fELF");
        header[4] = 2;
        header[5] = 1;
        header[18..20].copy_from_slice(&62u16.to_le_bytes());
        assert_eq!(
            linux::elf_id(&header),
            Some(linux::ElfId {
                class: 2,
                machine: 62
            })
        );

        header[4] = 1;
        header[18..20].copy_from_slice(&3u16.to_le_bytes());
        assert_eq!(
            linux::elf_id(&header),
            Some(linux::ElfId {
                class: 1,
                machine: 3
            })
        );

        header[4] = 3;
        assert_eq!(linux::elf_id(&header), None);
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
        let expected = linux::expected_id(&native).expect("el JRE expone su arquitectura");
        for library in [
            "libXtst.so.6",
            "libX11.so.6",
            "libXi.so.6",
            "libXext.so.6",
            "libXrender.so.1",
        ] {
            let dir = path
                .split(':')
                .find(|dir| std::path::Path::new(dir).join(library).is_file())
                .unwrap_or_else(|| panic!("{library} no está en {path}"));
            assert_eq!(
                linux::file_id(&std::path::Path::new(dir).join(library)),
                Some(expected),
                "{library} en {dir} no coincide con la arquitectura del JRE"
            );
        }
        assert_eq!(
            path.split(':').next_back(),
            Some("/run/current-system/sw/share/nix-ld/lib")
        );
    }

    fn write_elf(path: &std::path::Path, class: u8, machine: u16) {
        let mut header = [0u8; 64];
        header[..4].copy_from_slice(b"\x7fELF");
        header[4] = class;
        header[5] = 1;
        header[18..20].copy_from_slice(&machine.to_le_bytes());
        std::fs::write(path, header).unwrap();
    }

    fn elf64_x86_64() -> linux::ElfId {
        linux::ElfId {
            class: 2,
            machine: 62,
        }
    }

    #[cfg(unix)]
    #[test]
    fn nix_package_lookup_finds_the_library_by_package_name() {
        let temp = tempfile::tempdir().unwrap();
        let package = temp.path().join("abc-libxtst-1.2.6");
        std::fs::create_dir_all(package.join("lib")).unwrap();
        write_elf(&package.join("lib").join("libXtst.so.6"), 2, 62);
        let other = temp.path().join("zzz-libxi-1.8.2");
        std::fs::create_dir_all(other.join("lib")).unwrap();
        write_elf(&other.join("lib").join("libXi.so.6"), 2, 62);

        assert_eq!(
            linux::nix_package_dir_in(temp.path(), "libXtst.so.6", elf64_x86_64()),
            Some(package.join("lib").display().to_string())
        );
        assert_eq!(
            linux::nix_package_dir_in(temp.path(), "libXi.so.6", elf64_x86_64()),
            Some(other.join("lib").display().to_string())
        );
        assert_eq!(
            linux::nix_package_dir_in(temp.path(), "libX11.so.6", elf64_x86_64()),
            None
        );
    }

    #[cfg(unix)]
    #[test]
    fn nix_package_lookup_skips_libraries_of_the_wrong_architecture() {
        let temp = tempfile::tempdir().unwrap();
        let valid = temp.path().join("aaa-libxtst-1.2.6");
        std::fs::create_dir_all(valid.join("lib")).unwrap();
        write_elf(&valid.join("lib").join("libXtst.so.6"), 2, 62);
        let wrong = temp.path().join("zzz-libxtst-1.2.6");
        std::fs::create_dir_all(wrong.join("lib")).unwrap();
        write_elf(&wrong.join("lib").join("libXtst.so.6"), 1, 3);

        assert_eq!(
            linux::nix_package_dir_in(temp.path(), "libXtst.so.6", elf64_x86_64()),
            Some(valid.join("lib").display().to_string())
        );
    }
}
