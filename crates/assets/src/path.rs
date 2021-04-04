use std::{borrow::Cow, path::Path};

#[derive(Debug, Hash, Clone)]
pub struct AssetPath<'a> {
    path: Cow<'a, Path>,
    label: Option<Cow<'a, str>>,
}

impl<'a> AssetPath<'a> {
    pub fn new(path: Cow<'a, Path>, label: Option<Cow<'a, str>>) -> Self {
        Self { path, label }
    }

    pub fn from_path(path: &'a Path) -> Self {
        Self {
            path: Cow::Borrowed(path),
            label: None,
        }
    }

    /// Get a reference to the asset path's path.
    pub fn path(&self) -> &Cow<'a, Path> {
        &self.path
    }

    /// Get a reference to the asset path's label.
    pub fn label(&self) -> &Option<Cow<'a, str>> {
        &self.label
    }

    /// Set the asset path's label.
    pub fn set_label(&mut self, label: Option<Cow<'a, str>>) {
        self.label = label;
    }
}
