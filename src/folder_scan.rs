use {std::path::Path, walkdir::WalkDir};

pub fn walkdir(root: &Path) -> WalkDir {
    WalkDir::new(root).sort_by(|a, b| a.file_name().cmp(b.file_name()))
}
