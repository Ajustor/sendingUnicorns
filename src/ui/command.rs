use domain::{CollectionName, Place, RequestId};

/// What the user asked for, collected from widgets and shortcuts and run
/// once per frame by [`super::SendingApp`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Opens a request in a tab (or shows its tab).
    Open(RequestId),
    CloseTab(RequestId),
    CloseActiveTab,
    /// Shows the next (or previous) tab.
    CycleTabs(isize),
    MoveTab(RequestId, usize),
    /// An empty draft request in a new tab.
    NewTab,
    Send,
    CancelSend,
    /// Saves everything; a draft asks where to go first.
    Save,
    NewCollection,
    Import,
    /// Opens the paste-a-curl dialog.
    ImportCurl,
    CopyAsCurl,
    Export(CollectionName),
    RenameCollection(CollectionName),
    DeleteCollection(CollectionName),
    /// Auth, headers, variables and docs of a collection.
    CollectionSettings(CollectionName),
    /// Auth and headers of a folder.
    FolderSettings(RequestId),
    NewRequest {
        collection: CollectionName,
        folder: Option<RequestId>,
    },
    NewFolder {
        collection: CollectionName,
        parent: Option<RequestId>,
    },
    /// A request or a folder.
    Rename(RequestId),
    DuplicateRequest(RequestId),
    /// A request or a folder.
    Delete(RequestId),
    Move(RequestId, Place),
    SetEnvironment(CollectionName, Option<String>),
    ManageEnvironments(CollectionName),
    ManageGlobals,
    /// Sends every request of a collection (or folder) in order.
    Run {
        collection: CollectionName,
        folder: Option<RequestId>,
    },
    /// Reopens a history entry in a new tab.
    OpenHistory(usize),
    ClearHistory,
    OpenSettings,
    SaveResponseBody,
    CheckForUpdates,
    InstallUpdate,
    UpdateLater,
    SkipUpdate,
    RestartToUpdate,
}
