//! Collection use cases: load, create, save, rename, delete, import, export.

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use domain::{CollectionName, NameError, Workspace};

use crate::ports::{CollectionArchive, CollectionRepository, GlobalsRepository, StoreError};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CollectionError {
    Name(NameError),
    Store(StoreError),
    NotFound(CollectionName),
}

impl fmt::Display for CollectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name(e) => write!(f, "{e}"),
            Self::Store(e) => write!(f, "{e}"),
            Self::NotFound(name) => write!(f, "no collection named {name:?}"),
        }
    }
}

impl std::error::Error for CollectionError {}

impl From<NameError> for CollectionError {
    fn from(e: NameError) -> Self {
        Self::Name(e)
    }
}

impl From<StoreError> for CollectionError {
    fn from(e: StoreError) -> Self {
        Self::Store(e)
    }
}

/// Reads every saved collection at start-up.
#[derive(Debug, Clone)]
pub struct LoadWorkspace {
    repository: Arc<dyn CollectionRepository>,
}

impl LoadWorkspace {
    pub fn new(repository: Arc<dyn CollectionRepository>) -> Self {
        Self { repository }
    }

    /// The workspace, and what could not be read in it.
    ///
    /// # Errors
    ///
    /// When the storage itself can't be read.
    pub fn execute(&self) -> Result<(Workspace, Vec<StoreError>), StoreError> {
        let (collections, problems) = self.repository.load_all()?;
        Ok((Workspace::new(collections), problems))
    }
}

/// Creates an empty collection, saved at once.
#[derive(Debug, Clone)]
pub struct CreateCollection {
    repository: Arc<dyn CollectionRepository>,
}

impl CreateCollection {
    pub fn new(repository: Arc<dyn CollectionRepository>) -> Self {
        Self { repository }
    }

    /// # Errors
    ///
    /// When the name is invalid or taken, or the collection can't be saved
    /// (it then stays in the workspace, unsaved).
    pub fn execute(
        &self,
        workspace: &mut Workspace,
        name: &str,
    ) -> Result<CollectionName, CollectionError> {
        let name = CollectionName::parse(name)?;
        workspace.add_collection(name.clone())?;
        save(self.repository.as_ref(), workspace, &name)?;
        Ok(name)
    }
}

/// Writes the unsaved changes: collections and global variables.
#[derive(Debug, Clone)]
pub struct SaveCollections {
    repository: Arc<dyn CollectionRepository>,
    globals: Arc<dyn GlobalsRepository>,
}

impl SaveCollections {
    pub fn new(
        repository: Arc<dyn CollectionRepository>,
        globals: Arc<dyn GlobalsRepository>,
    ) -> Self {
        Self {
            repository,
            globals,
        }
    }

    /// Saves every collection with unsaved changes, and the globals if they
    /// changed; the failures (the rest is still saved).
    pub fn execute(&self, workspace: &mut Workspace) -> Vec<CollectionError> {
        let mut failures: Vec<CollectionError> = workspace
            .dirty()
            .iter()
            .filter_map(|name| save(self.repository.as_ref(), workspace, name).err())
            .collect();
        if workspace.globals_dirty() {
            match self.globals.save(workspace.globals()) {
                Ok(()) => workspace.mark_globals_saved(),
                Err(e) => failures.push(CollectionError::Store(e)),
            }
        }
        failures
    }
}

#[derive(Debug, Clone)]
pub struct RenameCollection {
    repository: Arc<dyn CollectionRepository>,
}

impl RenameCollection {
    pub fn new(repository: Arc<dyn CollectionRepository>) -> Self {
        Self { repository }
    }

    /// Saves under the new name, then deletes the old file.
    ///
    /// # Errors
    ///
    /// When the new name is invalid or taken, or the files can't be written.
    pub fn execute(
        &self,
        workspace: &mut Workspace,
        from: &CollectionName,
        to: &str,
    ) -> Result<CollectionName, CollectionError> {
        let to = CollectionName::parse(to)?;
        if workspace.collection(from).is_none() {
            return Err(CollectionError::NotFound(from.clone()));
        }
        if &to == from {
            return Ok(to);
        }
        workspace.rename_collection(from, to.clone())?;
        save(self.repository.as_ref(), workspace, &to)?;
        self.repository.delete(from)?;
        Ok(to)
    }
}

#[derive(Debug, Clone)]
pub struct DeleteCollection {
    repository: Arc<dyn CollectionRepository>,
}

impl DeleteCollection {
    pub fn new(repository: Arc<dyn CollectionRepository>) -> Self {
        Self { repository }
    }

    /// # Errors
    ///
    /// When the file can't be deleted (the collection then stays).
    pub fn execute(
        &self,
        workspace: &mut Workspace,
        name: &CollectionName,
    ) -> Result<(), CollectionError> {
        self.repository.delete(name)?;
        workspace.remove_collection(name);
        Ok(())
    }
}

/// Adds a collection from a file (ours, an older version's, Postman's).
#[derive(Debug, Clone)]
pub struct ImportCollection {
    archive: Arc<dyn CollectionArchive>,
    repository: Arc<dyn CollectionRepository>,
}

impl ImportCollection {
    pub fn new(
        archive: Arc<dyn CollectionArchive>,
        repository: Arc<dyn CollectionRepository>,
    ) -> Self {
        Self {
            archive,
            repository,
        }
    }

    /// The name it got: its own, or `name (2)` when taken.
    ///
    /// # Errors
    ///
    /// When the file can't be read, or the collection can't be saved (it
    /// then stays in the workspace, unsaved).
    pub fn execute(
        &self,
        workspace: &mut Workspace,
        path: &Path,
    ) -> Result<CollectionName, CollectionError> {
        let collection = self.archive.read(path)?;
        let name = workspace.import(collection);
        save(self.repository.as_ref(), workspace, &name)?;
        Ok(name)
    }
}

/// Writes a collection to a file of the user's choice, to share it.
#[derive(Debug, Clone)]
pub struct ExportCollection {
    archive: Arc<dyn CollectionArchive>,
}

impl ExportCollection {
    pub fn new(archive: Arc<dyn CollectionArchive>) -> Self {
        Self { archive }
    }

    /// Unsaved changes are exported too.
    ///
    /// # Errors
    ///
    /// When there is no such collection or the file can't be written.
    pub fn execute(
        &self,
        workspace: &Workspace,
        name: &CollectionName,
        path: &Path,
    ) -> Result<(), CollectionError> {
        let collection = workspace
            .collection(name)
            .ok_or_else(|| CollectionError::NotFound(name.clone()))?;
        Ok(self.archive.write(path, collection)?)
    }
}

fn save(
    repository: &dyn CollectionRepository,
    workspace: &mut Workspace,
    name: &CollectionName,
) -> Result<(), CollectionError> {
    let collection = workspace
        .collection(name)
        .ok_or_else(|| CollectionError::NotFound(name.clone()))?;
    repository.save(collection)?;
    workspace.mark_saved(name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::MemoryGlobals;
    use crate::fakes::{FakeArchive, MemoryRepository};
    use domain::{Collection, Item, KeyValue, Request};

    fn name(s: &str) -> CollectionName {
        CollectionName::parse(s).unwrap()
    }

    #[test]
    fn create_saves_at_once() {
        let repo = Arc::new(MemoryRepository::default());
        let mut ws = Workspace::default();
        let created = CreateCollection::new(repo.clone())
            .execute(&mut ws, " api ")
            .unwrap();
        assert_eq!(created, name("api"));
        assert!(!ws.has_unsaved_changes());
        assert_eq!(repo.names(), ["api"]);
        assert!(matches!(
            CreateCollection::new(repo).execute(&mut ws, "a/b"),
            Err(CollectionError::Name(NameError::Forbidden('/')))
        ));
    }

    #[test]
    fn save_writes_only_unsaved_collections() {
        let repo = Arc::new(MemoryRepository::default());
        let mut ws = Workspace::new(vec![Collection::new(name("a")), Collection::new(name("b"))]);
        ws.edit(&name("b"))
            .unwrap()
            .items
            .push(Item::Request(Request::new("r")));
        ws.edit_globals().0.push(KeyValue::new("g", "1"));
        let globals = Arc::new(MemoryGlobals::default());
        assert!(
            SaveCollections::new(repo.clone(), globals.clone())
                .execute(&mut ws)
                .is_empty()
        );
        assert_eq!(repo.names(), ["b"]);
        assert_eq!(globals.saved().0.len(), 1);
        assert!(!ws.has_unsaved_changes());
    }

    #[test]
    fn a_failed_save_stays_unsaved() {
        let repo = Arc::new(MemoryRepository::failing());
        let mut ws = Workspace::default();
        assert!(
            CreateCollection::new(repo.clone())
                .execute(&mut ws, "api")
                .is_err()
        );
        assert!(ws.is_dirty(&name("api")));
        assert_eq!(
            SaveCollections::new(repo, Arc::new(MemoryGlobals::default()))
                .execute(&mut ws)
                .len(),
            1
        );
    }

    #[test]
    fn rename_moves_the_file() {
        let repo = Arc::new(MemoryRepository::default());
        let mut ws = Workspace::default();
        CreateCollection::new(repo.clone())
            .execute(&mut ws, "old")
            .unwrap();
        RenameCollection::new(repo.clone())
            .execute(&mut ws, &name("old"), "new")
            .unwrap();
        assert_eq!(repo.names(), ["new"]);
        assert!(ws.collection(&name("new")).is_some());
    }

    #[test]
    fn delete_removes_file_and_collection() {
        let repo = Arc::new(MemoryRepository::default());
        let mut ws = Workspace::default();
        CreateCollection::new(repo.clone())
            .execute(&mut ws, "api")
            .unwrap();
        DeleteCollection::new(repo.clone())
            .execute(&mut ws, &name("api"))
            .unwrap();
        assert!(repo.names().is_empty());
        assert!(ws.collections().is_empty());
    }

    #[test]
    fn import_takes_a_free_name_and_export_writes_the_file() {
        let repo = Arc::new(MemoryRepository::default());
        let archive = Arc::new(FakeArchive::with(Collection::new(name("api"))));
        let mut ws = Workspace::new(vec![Collection::new(name("api"))]);
        let imported = ImportCollection::new(archive.clone(), repo.clone())
            .execute(&mut ws, Path::new("in.json"))
            .unwrap();
        assert_eq!(imported, name("api (2)"));
        assert_eq!(repo.names(), ["api (2)"]);

        ExportCollection::new(archive.clone())
            .execute(&ws, &imported, Path::new("out.json"))
            .unwrap();
        assert_eq!(archive.written(), ["api (2)"]);
        assert!(matches!(
            ExportCollection::new(archive).execute(&ws, &name("nope"), Path::new("x")),
            Err(CollectionError::NotFound(_))
        ));
    }
}
