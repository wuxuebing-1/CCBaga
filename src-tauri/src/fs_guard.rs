// [INPUT]: Local filesystem paths and metadata on supported desktop platforms.
// [OUTPUT]: Reparse/link rejection, ownership checks and atomic replacement.
// [POS]: Shared filesystem safety boundary; no discovery or deletion policy.
use std::{fs, path::Path};

pub(crate) fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

pub(crate) fn owned(path: &Path, meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        let _ = meta;
        crate::windows::owned_by_current_user(path)
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        unsafe extern "C" {
            fn getuid() -> u32;
        }
        let _ = path;
        meta.uid() == unsafe { getuid() }
    }
}

pub(crate) fn real_path(path: &Path) -> bool {
    if !path.is_absolute()
        || path.components().any(|p| {
            matches!(
                p,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return false;
    }
    // A junction at ANY ancestor can redirect an otherwise ordinary child.
    #[cfg(windows)]
    {
        for ancestor in path.ancestors().filter(|p| p.parent().is_some()) {
            if fs::symlink_metadata(ancestor)
                .map(|m| is_link(&m))
                .unwrap_or(true)
            {
                return false;
            }
        }
        fs::canonicalize(path).is_ok()
    }
    #[cfg(not(windows))]
    {
        fs::canonicalize(path).ok().as_deref() == Some(path)
    }
}

pub(crate) fn replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;
        if !destination.exists() {
            return fs::rename(source, destination);
        }
        let src: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
        let dst: Vec<u16> = destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        // ReplaceFile preserves the destination ACL; the temporary file is never a backup.
        if unsafe {
            ReplaceFileW(
                dst.as_ptr(),
                src.as_ptr(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        fs::rename(source, destination)
    }
}
