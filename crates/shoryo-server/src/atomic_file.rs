use std::io::{self, Write};
use std::path::Path;

pub(crate) fn write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let directory = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    temporary.write_all(contents)?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}
