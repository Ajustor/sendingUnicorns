//! A collection: requests in folders, the environments they run against,
//! its own variables, auth and headers, saved together.

use std::fmt;

use crate::auth::Auth;
use crate::environment::Environment;
use crate::key_value::{KeyValue, KeyValues};
use crate::request::{Request, RequestId};

/// Why a name was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NameError {
    Empty,
    TooLong,
    /// A character file systems refuse (`/`, `:`, `?`…).
    Forbidden(char),
    /// A device name Windows reserves (`CON`, `NUL`, `COM1`…).
    Reserved,
    /// Another collection (or environment) already has it.
    Taken,
}

impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("the name is empty"),
            Self::TooLong => write!(f, "the name is longer than {MAX_NAME} characters"),
            Self::Forbidden(c) => write!(f, "the name cannot contain {c:?}"),
            Self::Reserved => f.write_str("this name is reserved by the system"),
            Self::Taken => f.write_str("this name is already used"),
        }
    }
}

impl std::error::Error for NameError {}

const MAX_NAME: usize = 100;

/// A collection's name, which is also its file's: valid on every system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CollectionName(String);

impl CollectionName {
    /// # Errors
    ///
    /// When `name` would not make a file name on Windows, macOS or Linux.
    pub fn parse(name: &str) -> Result<Self, NameError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(NameError::Empty);
        }
        if name.chars().count() > MAX_NAME {
            return Err(NameError::TooLong);
        }
        if let Some(c) = name
            .chars()
            .find(|c| c.is_control() || r#"<>:"/\|?*"#.contains(*c))
        {
            return Err(NameError::Forbidden(c));
        }
        if name.ends_with('.') {
            return Err(NameError::Forbidden('.'));
        }
        let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
        let reserved = ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
            || ((stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.len() == 4
                && stem.as_bytes()[3].is_ascii_digit());
        if reserved {
            return Err(NameError::Reserved);
        }
        Ok(Self(name.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CollectionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A folder: requests and subfolders, with the auth and headers they
/// inherit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    pub id: RequestId,
    pub name: String,
    pub items: Vec<Item>,
    pub auth: Auth,
    pub headers: KeyValues,
}

impl Folder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: RequestId::next(),
            name: name.into(),
            items: Vec::new(),
            auth: Auth::Inherit,
            headers: KeyValues::default(),
        }
    }
}

// A request is larger than a folder; boxing it would cost an indirection
// on every lookup of the tree for a few bytes saved (R-VER-10).
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Request(Request),
    Folder(Folder),
}

impl Item {
    pub fn id(&self) -> RequestId {
        match self {
            Self::Request(r) => r.id,
            Self::Folder(f) => f.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Request(r) => &r.name,
            Self::Folder(f) => &f.name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collection {
    pub name: CollectionName,
    pub items: Vec<Item>,
    pub environments: Vec<Environment>,
    /// Shared by every environment; an environment's variable wins.
    pub variables: KeyValues,
    /// What requests that inherit use; [`Auth::Inherit`] here means none.
    pub auth: Auth,
    /// Sent with every request, before its own headers.
    pub headers: KeyValues,
    pub docs: String,
}

impl Collection {
    pub fn new(name: CollectionName) -> Self {
        Self {
            name,
            items: Vec::new(),
            environments: Vec::new(),
            variables: KeyValues::default(),
            auth: Auth::None,
            headers: KeyValues::default(),
            docs: String::new(),
        }
    }

    /// Every request, depth first, in order (as a runner sends them).
    pub fn requests(&self) -> Vec<&Request> {
        let mut out = Vec::new();
        collect_requests(&self.items, &mut out);
        out
    }

    /// The requests of folder `id` (all of them when `None`), in order.
    pub fn requests_in(&self, folder: Option<RequestId>) -> Vec<&Request> {
        match folder.and_then(|id| self.folder(id)) {
            Some(f) => {
                let mut out = Vec::new();
                collect_requests(&f.items, &mut out);
                out
            }
            None => self.requests(),
        }
    }

    pub fn request(&self, id: RequestId) -> Option<&Request> {
        find(&self.items, id).and_then(|item| match item {
            Item::Request(r) => Some(r),
            Item::Folder(_) => None,
        })
    }

    pub fn request_mut(&mut self, id: RequestId) -> Option<&mut Request> {
        find_mut(&mut self.items, id).and_then(|item| match item {
            Item::Request(r) => Some(r),
            Item::Folder(_) => None,
        })
    }

    pub fn folder(&self, id: RequestId) -> Option<&Folder> {
        find(&self.items, id).and_then(|item| match item {
            Item::Folder(f) => Some(f),
            Item::Request(_) => None,
        })
    }

    pub fn folder_mut(&mut self, id: RequestId) -> Option<&mut Folder> {
        find_mut(&mut self.items, id).and_then(|item| match item {
            Item::Folder(f) => Some(f),
            Item::Request(_) => None,
        })
    }

    /// Whether a request or folder `id` is in this collection.
    pub fn contains(&self, id: RequestId) -> bool {
        find(&self.items, id).is_some()
    }

    /// The folders around `id`, outermost first.
    pub fn ancestors(&self, id: RequestId) -> Vec<&Folder> {
        fn walk<'a>(items: &'a [Item], id: RequestId, path: &mut Vec<&'a Folder>) -> bool {
            for item in items {
                if item.id() == id {
                    return true;
                }
                if let Item::Folder(f) = item {
                    path.push(f);
                    if walk(&f.items, id, path) {
                        return true;
                    }
                    path.pop();
                }
            }
            false
        }
        let mut path = Vec::new();
        walk(&self.items, id, &mut path);
        path
    }

    /// The auth request `id` really uses: its own, else its innermost
    /// folder's, else the collection's.
    pub fn effective_auth(&self, id: RequestId) -> Auth {
        let own = self.request(id).map(|r| r.auth.clone()).unwrap_or_default();
        if own != Auth::Inherit {
            return own;
        }
        self.ancestors(id)
            .into_iter()
            .rev()
            .map(|f| &f.auth)
            .find(|a| **a != Auth::Inherit)
            .unwrap_or(&self.auth)
            .clone()
    }

    /// The headers request `id` inherits: the collection's, then each
    /// folder's, outermost first.
    pub fn inherited_headers(&self, id: RequestId) -> Vec<KeyValue> {
        self.headers
            .active()
            .chain(
                self.ancestors(id)
                    .into_iter()
                    .flat_map(|f| f.headers.active()),
            )
            .cloned()
            .collect()
    }

    /// Removes a request or a folder (with its content).
    pub fn remove(&mut self, id: RequestId) -> Option<Item> {
        fn walk(items: &mut Vec<Item>, id: RequestId) -> Option<Item> {
            if let Some(i) = items.iter().position(|item| item.id() == id) {
                return Some(items.remove(i));
            }
            items.iter_mut().find_map(|item| match item {
                Item::Folder(f) => walk(&mut f.items, id),
                Item::Request(_) => None,
            })
        }
        walk(&mut self.items, id)
    }

    /// Puts `item` in folder `parent` (the collection's root when `None`
    /// or unknown), at `index` (the end when `None` or past it).
    pub fn insert(&mut self, item: Item, parent: Option<RequestId>, index: Option<usize>) {
        let items = match parent.and_then(|id| self.folder_mut(id)) {
            Some(f) => &mut f.items,
            None => &mut self.items,
        };
        let index = index.unwrap_or(items.len()).min(items.len());
        items.insert(index, item);
    }

    /// Where `id` is: its folder (`None` at the root) and index there.
    pub fn position(&self, id: RequestId) -> Option<(Option<RequestId>, usize)> {
        let parent = self.ancestors(id).last().map(|f| f.id);
        let items = match parent {
            Some(p) => &self.folder(p)?.items,
            None => &self.items,
        };
        Some((parent, items.iter().position(|i| i.id() == id)?))
    }

    /// Puts a copy of request `id` right after it; the copy's id.
    pub fn duplicate_request(&mut self, id: RequestId) -> Option<RequestId> {
        let original = self.request(id)?;
        let copy = original.duplicate(format!("{} copy", original.name));
        let copy_id = copy.id;
        let (parent, index) = self.position(id)?;
        self.insert(Item::Request(copy), parent, Some(index + 1));
        Some(copy_id)
    }

    pub fn environment(&self, name: &str) -> Option<&Environment> {
        self.environments.iter().find(|e| e.name == name)
    }

    pub fn environment_mut(&mut self, name: &str) -> Option<&mut Environment> {
        self.environments.iter_mut().find(|e| e.name == name)
    }

    /// # Errors
    ///
    /// When the name is empty or another environment has it.
    pub fn add_environment(&mut self, name: &str) -> Result<(), NameError> {
        let name = self.free_environment_name(name, None)?;
        self.environments.push(Environment::new(name));
        Ok(())
    }

    /// # Errors
    ///
    /// When the new name is empty or another environment has it.
    pub fn rename_environment(&mut self, from: &str, to: &str) -> Result<(), NameError> {
        let to = self.free_environment_name(to, Some(from))?;
        if let Some(env) = self.environment_mut(from) {
            env.name = to;
        }
        Ok(())
    }

    pub fn remove_environment(&mut self, name: &str) {
        self.environments.retain(|e| e.name != name);
    }

    fn free_environment_name(
        &self,
        name: &str,
        current: Option<&str>,
    ) -> Result<String, NameError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(NameError::Empty);
        }
        if Some(name) != current && self.environment(name).is_some() {
            return Err(NameError::Taken);
        }
        Ok(name.to_owned())
    }
}

fn collect_requests<'a>(items: &'a [Item], out: &mut Vec<&'a Request>) {
    for item in items {
        match item {
            Item::Request(r) => out.push(r),
            Item::Folder(f) => collect_requests(&f.items, out),
        }
    }
}

fn find(items: &[Item], id: RequestId) -> Option<&Item> {
    items.iter().find_map(|item| {
        if item.id() == id {
            return Some(item);
        }
        match item {
            Item::Folder(f) => find(&f.items, id),
            Item::Request(_) => None,
        }
    })
}

fn find_mut(items: &mut [Item], id: RequestId) -> Option<&mut Item> {
    for item in items {
        if item.id() == id {
            return Some(item);
        }
        if let Item::Folder(f) = item
            && let Some(found) = find_mut(&mut f.items, id)
        {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(s: &str) -> CollectionName {
        CollectionName::parse(s).unwrap()
    }

    #[test]
    fn names_must_make_file_names() {
        assert_eq!(name("  My API ").as_str(), "My API");
        assert_eq!(CollectionName::parse(" "), Err(NameError::Empty));
        assert_eq!(CollectionName::parse("a/b"), Err(NameError::Forbidden('/')));
        assert_eq!(
            CollectionName::parse("end."),
            Err(NameError::Forbidden('.'))
        );
        assert_eq!(CollectionName::parse("nul"), Err(NameError::Reserved));
        assert_eq!(CollectionName::parse("COM3.json"), Err(NameError::Reserved));
        assert!(CollectionName::parse("COMPANY").is_ok());
        assert_eq!(
            CollectionName::parse(&"x".repeat(101)),
            Err(NameError::TooLong)
        );
    }

    /// `root`, `users/{list, admin/{promote}}`.
    fn tree() -> (Collection, RequestId, RequestId, RequestId, RequestId) {
        let mut c = Collection::new(name("c"));
        let root = Request::new("root");
        let list = Request::new("list");
        let promote = Request::new("promote");
        let mut admin = Folder::new("admin");
        let (root_id, list_id, promote_id, admin_id) = (root.id, list.id, promote.id, admin.id);
        admin.items.push(Item::Request(promote));
        let mut users = Folder::new("users");
        users.items = vec![Item::Request(list), Item::Folder(admin)];
        c.items = vec![Item::Request(root), Item::Folder(users)];
        (c, root_id, list_id, promote_id, admin_id)
    }

    #[test]
    fn finds_requests_in_folders() {
        let (c, root, list, promote, admin) = tree();
        let names: Vec<_> = c.requests().iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["root", "list", "promote"]);
        assert_eq!(c.request(promote).unwrap().name, "promote");
        assert!(c.request(admin).is_none() && c.folder(admin).is_some());
        let path: Vec<_> = c
            .ancestors(promote)
            .iter()
            .map(|f| f.name.as_str())
            .collect();
        assert_eq!(path, ["users", "admin"]);
        assert!(c.ancestors(root).is_empty());
        assert_eq!(c.requests_in(Some(admin)).len(), 1);
        assert_eq!(
            c.position(list),
            Some((c.ancestors(list).last().map(|f| f.id), 0))
        );
    }

    #[test]
    fn auth_and_headers_are_inherited() {
        let (mut c, root, list, promote, admin) = tree();
        c.auth = Auth::Bearer {
            token: "collection".into(),
        };
        c.headers.0.push(KeyValue::new("X-App", "su"));
        let users = c.ancestors(list)[0].id;
        c.folder_mut(users)
            .unwrap()
            .headers
            .0
            .push(KeyValue::new("X-Folder", "users"));
        c.folder_mut(admin).unwrap().auth = Auth::Basic {
            username: "a".into(),
            password: "b".into(),
        };
        assert_eq!(
            c.effective_auth(root),
            Auth::Bearer {
                token: "collection".into()
            }
        );
        assert_eq!(
            c.effective_auth(list),
            Auth::Bearer {
                token: "collection".into()
            }
        );
        assert!(matches!(c.effective_auth(promote), Auth::Basic { .. }));
        c.request_mut(promote).unwrap().auth = Auth::None;
        assert_eq!(c.effective_auth(promote), Auth::None);
        let keys: Vec<_> = c
            .inherited_headers(promote)
            .into_iter()
            .map(|kv| kv.key)
            .collect();
        assert_eq!(keys, ["X-App", "X-Folder"]);
    }

    #[test]
    fn moves_duplicates_and_removes() {
        let (mut c, root, list, _, admin) = tree();
        let copy = c.duplicate_request(list).unwrap();
        let names: Vec<_> = c.requests().iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["root", "list", "list copy", "promote"]);
        let item = c.remove(root).unwrap();
        c.insert(item, Some(admin), Some(0));
        assert_eq!(c.ancestors(root).len(), 2);
        assert!(c.remove(admin).is_some());
        assert!(!c.contains(root));
        assert!(c.contains(copy));
    }

    #[test]
    fn environment_names_are_unique() {
        let mut c = Collection::new(name("c"));
        c.add_environment("dev").unwrap();
        c.add_environment("prod").unwrap();
        assert_eq!(c.add_environment(" dev "), Err(NameError::Taken));
        assert_eq!(c.rename_environment("dev", "prod"), Err(NameError::Taken));
        c.rename_environment("dev", "dev").unwrap();
        c.rename_environment("dev", "local").unwrap();
        assert!(c.environment("local").is_some());
        c.remove_environment("local");
        assert_eq!(c.environments.len(), 1);
    }
}
