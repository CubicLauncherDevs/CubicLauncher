pub mod auth;
pub mod core;
pub mod download;
pub mod fs;
pub mod instance;

pub use auth::AuthError;
pub use core::CoreError;
pub use download::DownloadError;
pub use fs::FsError;
pub use instance::InstanceError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Instance(#[from] InstanceError),

    #[error(transparent)]
    CoreError(#[from] CoreError),

    #[error(transparent)]
    Auth(#[from] AuthError),

    #[error(transparent)]
    Download(#[from] DownloadError),

    #[error(transparent)]
    Fs(#[from] FsError),
}

fn json_error(code: &str, params: &[(&str, String)]) -> String {
    if params.is_empty() {
        return format!(r#"{{"code":"{}"}}"#, code);
    }
    let inner: Vec<String> = params
        .iter()
        .map(|(k, v)| {
            format!(
                r#""{}":"{}""#,
                k,
                v.replace('\\', "\\\\").replace('"', "\\\"")
            )
        })
        .collect();
    format!(r#"{{"code":"{}","params":{{{}}}}}"#, code, inner.join(","))
}

impl AppError {
    pub fn to_json(&self) -> String {
        let (code, params) = match self {
            Self::Instance(e) => (e.code(), e.params()),
            Self::CoreError(e) => (e.code(), e.params()),
            Self::Auth(e) => (e.code(), e.params()),
            Self::Download(e) => (e.code(), e.params()),
            Self::Fs(e) => (e.code(), e.params()),
        };
        json_error(code, &params)
    }
}

impl From<AppError> for String {
    fn from(e: AppError) -> String {
        e.to_json()
    }
}

impl From<InstanceError> for String {
    fn from(e: InstanceError) -> String {
        json_error(e.code(), &e.params())
    }
}

impl From<CoreError> for String {
    fn from(e: CoreError) -> String {
        json_error(e.code(), &e.params())
    }
}

impl From<AuthError> for String {
    fn from(e: AuthError) -> String {
        json_error(e.code(), &e.params())
    }
}

impl From<DownloadError> for String {
    fn from(e: DownloadError) -> String {
        json_error(e.code(), &e.params())
    }
}

impl From<FsError> for String {
    fn from(e: FsError) -> String {
        json_error(e.code(), &e.params())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_error_not_found() {
        let s: String = AppError::Instance(InstanceError::NotFound).into();
        assert_eq!(s, r#"{"code":"INST_NOT_FOUND"}"#);
    }

    #[test]
    fn test_app_error_with_params() {
        let s: String = AppError::Instance(InstanceError::JreNotFound("17".into())).into();
        assert_eq!(
            s,
            r#"{"code":"INST_JRE_MISSING","params":{"version":"17"}}"#
        );
    }

    #[test]
    fn test_fs_error() {
        let s: String = AppError::Fs(FsError::NotFound("/x".into())).into();
        assert_eq!(s, r#"{"code":"FS_NOT_FOUND","params":{"path":"/x"}}"#);
    }

    #[test]
    fn test_auth_error() {
        let s: String = AppError::Auth(AuthError::SaveTokensFailed("oops".into())).into();
        assert_eq!(
            s,
            r#"{"code":"AUTH_TOKENS_SAVE","params":{"error":"oops"}}"#
        );
    }

    #[test]
    fn test_download_error() {
        let s: String = AppError::Download(DownloadError::NoFabricLoader).into();
        assert_eq!(s, r#"{"code":"DL_NO_FABRIC"}"#);
    }

    #[test]
    fn test_core_error() {
        let s: String = CoreError::LockPoisoned("oh no".into()).into();
        assert_eq!(s, r#"{"code":"CORE_LOCK","params":{"error":"oh no"}}"#);
    }

    #[test]
    fn test_instance_error_into_string() {
        let s: String = InstanceError::NotFound.into();
        assert_eq!(s, r#"{"code":"INST_NOT_FOUND"}"#);
    }

    #[test]
    fn test_fs_nested_in_instance() {
        let s: String =
            AppError::Instance(InstanceError::Fs(FsError::NotFound("/tmp".into()))).into();
        assert_eq!(s, r#"{"code":"FS_NOT_FOUND","params":{"path":"/tmp"}}"#);
    }

    #[test]
    fn test_json_escaping() {
        let s: String = InstanceError::InstNameParse(r#"bad"name"#.into()).into();
        assert!(s.contains(r#"bad\"name"#));
    }
}
