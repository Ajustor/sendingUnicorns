//! Everything open: the collections (and which have unsaved changes),
//! unsaved draft requests, the active environments, global and runtime
//! variables.

use std::collections::{BTreeSet, HashMap};

use crate::collection::{Collection, CollectionName, Folder, Item, NameError};
use crate::environment::Environment;
use crate::key_value::KeyValues;
use crate::request::{Request, RequestId};
use crate::template::Variables;

/// A request, and the collection it belongs to (`None` for a draft).
#[derive(Debug, Clone, Copy)]
pub struct RequestRef<'a> {
    pub collection: Option<&'a Collection>,
    pub request: &'a Request,
}

/// Where an item goes: a collection, a folder in it (its root when
/// `None`), an index there (the end when `None`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub collection: CollectionName,
    pub folder: Option<RequestId>,
    pub index: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct Workspace {
    /// Sorted by name.
    collections: Vec<Collection>,
    /// Requests in no collection yet (new tabs, history, cURL imports).
    drafts: Vec<Request>,
    dirty: BTreeSet<CollectionName>,
    active_environments: HashMap<CollectionName, String>,
    /// Seen by every collection; saved apart from them.
    globals: KeyValues,
    globals_dirty: bool,
    /// Set by requests' post-response variables; gone when the app closes.
    runtime: Vec<(String, String)>,
}

impl Workspace {
    pub fn new(mut collections: Vec<Collection>) -> Self {
        collections.sort_by(|a, b| a.name.cmp(&b.name));
        Self {
            collections,
            ..Self::default()
        }
    }

    pub fn collections(&self) -> &[Collection] {
        &self.collections
    }

    pub fn collection(&self, name: &CollectionName) -> Option<&Collection> {
        self.collections.iter().find(|c| &c.name == name)
    }

    /// The collection, to change: it is marked unsaved.
    pub fn edit(&mut self, name: &CollectionName) -> Option<&mut Collection> {
        let collection = self.collections.iter_mut().find(|c| &c.name == name)?;
        self.dirty.insert(name.clone());
        Some(collection)
    }

    pub fn find(&self, id: RequestId) -> Option<RequestRef<'_>> {
        if let Some(request) = self.drafts.iter().find(|r| r.id == id) {
            return Some(RequestRef {
                collection: None,
                request,
            });
        }
        self.collections.iter().find_map(|collection| {
            collection.request(id).map(|request| RequestRef {
                collection: Some(collection),
                request,
            })
        })
    }

    /// The collection holding request or folder `id`.
    pub fn collection_of(&self, id: RequestId) -> Option<&Collection> {
        self.collections.iter().find(|c| c.contains(id))
    }

    /// The request, to change: its collection is marked unsaved.
    pub fn edit_request(&mut self, id: RequestId) -> Option<&mut Request> {
        if let Some(i) = self.drafts.iter().position(|r| r.id == id) {
            return self.drafts.get_mut(i);
        }
        let collection = self.collections.iter_mut().find(|c| c.contains(id))?;
        self.dirty.insert(collection.name.clone());
        collection.request_mut(id)
    }

    /// The folder, to change: its collection is marked unsaved.
    pub fn edit_folder(&mut self, id: RequestId) -> Option<&mut Folder> {
        let collection = self.collections.iter_mut().find(|c| c.contains(id))?;
        self.dirty.insert(collection.name.clone());
        collection.folder_mut(id)
    }

    pub fn drafts(&self) -> &[Request] {
        &self.drafts
    }

    /// A new request in no collection; its id.
    pub fn add_draft(&mut self, request: Request) -> RequestId {
        let id = request.id;
        self.drafts.push(request);
        id
    }

    pub fn is_draft(&self, id: RequestId) -> bool {
        self.drafts.iter().any(|r| r.id == id)
    }

    /// Removes a request or folder from wherever it is.
    pub fn remove(&mut self, id: RequestId) -> Option<Item> {
        if let Some(i) = self.drafts.iter().position(|r| r.id == id) {
            return Some(Item::Request(self.drafts.remove(i)));
        }
        let collection = self.collections.iter_mut().find(|c| c.contains(id))?;
        self.dirty.insert(collection.name.clone());
        collection.remove(id)
    }

    /// Moves a request or folder (a draft too) to `place`. Nothing moves
    /// into itself, nor into a collection that doesn't exist.
    pub fn move_to(&mut self, id: RequestId, place: &Place) -> bool {
        let Some(target) = self.collection(&place.collection) else {
            return false;
        };
        if let Some(folder) = place.folder {
            let inside_itself = folder == id || target.ancestors(folder).iter().any(|f| f.id == id);
            if inside_itself {
                return false;
            }
        }
        // Moving down in the same parent: the index counts the item itself.
        let mut index = place.index;
        if let Some(from) = self.collection_of(id)
            && from.name == place.collection
            && let Some((parent, at)) = from.position(id)
            && parent == place.folder
            && let Some(i) = index
            && i > at
        {
            index = Some(i - 1);
        }
        let Some(item) = self.remove(id) else {
            return false;
        };
        if let Some(collection) = self.edit(&place.collection) {
            collection.insert(item, place.folder, index);
        }
        true
    }

    /// A new folder; its id.
    pub fn add_folder(
        &mut self,
        collection: &CollectionName,
        parent: Option<RequestId>,
        name: &str,
    ) -> Option<RequestId> {
        let folder = Folder::new(name.trim());
        let id = folder.id;
        self.edit(collection)?
            .insert(Item::Folder(folder), parent, None);
        Some(id)
    }

    /// # Errors
    ///
    /// [`NameError::Taken`] when a collection already has this name.
    pub fn add_collection(&mut self, name: CollectionName) -> Result<(), NameError> {
        if self.collection(&name).is_some() {
            return Err(NameError::Taken);
        }
        self.dirty.insert(name.clone());
        self.insert_sorted(Collection::new(name));
        Ok(())
    }

    /// Adds an imported collection under a free name (`name (2)`…); the
    /// name it got.
    pub fn import(&mut self, mut collection: Collection) -> CollectionName {
        collection.name = self.free_name(&collection.name);
        let name = collection.name.clone();
        self.dirty.insert(name.clone());
        self.insert_sorted(collection);
        name
    }

    pub fn remove_collection(&mut self, name: &CollectionName) -> Option<Collection> {
        let index = self.collections.iter().position(|c| &c.name == name)?;
        self.dirty.remove(name);
        self.active_environments.remove(name);
        Some(self.collections.remove(index))
    }

    /// # Errors
    ///
    /// [`NameError::Taken`] when another collection has the new name.
    pub fn rename_collection(
        &mut self,
        from: &CollectionName,
        to: CollectionName,
    ) -> Result<(), NameError> {
        if from == &to {
            return Ok(());
        }
        if self.collection(&to).is_some() {
            return Err(NameError::Taken);
        }
        let active = self.active_environments.get(from).cloned();
        let Some(mut collection) = self.remove_collection(from) else {
            return Ok(());
        };
        collection.name = to.clone();
        if let Some(env) = active {
            self.active_environments.insert(to.clone(), env);
        }
        self.dirty.insert(to);
        self.insert_sorted(collection);
        Ok(())
    }

    pub fn is_dirty(&self, name: &CollectionName) -> bool {
        self.dirty.contains(name)
    }

    pub fn has_unsaved_changes(&self) -> bool {
        !self.dirty.is_empty() || self.globals_dirty
    }

    /// The collections with unsaved changes.
    pub fn dirty(&self) -> Vec<CollectionName> {
        self.dirty.iter().cloned().collect()
    }

    pub fn mark_saved(&mut self, name: &CollectionName) {
        self.dirty.remove(name);
    }

    pub fn globals(&self) -> &KeyValues {
        &self.globals
    }

    /// Loaded at start-up: not a change.
    pub fn set_globals(&mut self, globals: KeyValues) {
        self.globals = globals;
    }

    /// The global variables, to change: marked unsaved.
    pub fn edit_globals(&mut self) -> &mut KeyValues {
        self.globals_dirty = true;
        &mut self.globals
    }

    pub fn globals_dirty(&self) -> bool {
        self.globals_dirty
    }

    pub fn mark_globals_saved(&mut self) {
        self.globals_dirty = false;
    }

    pub fn runtime(&self) -> &[(String, String)] {
        &self.runtime
    }

    /// Sets a runtime variable (a request's post-response variable).
    pub fn set_runtime(&mut self, name: &str, value: String) {
        match self.runtime.iter_mut().find(|(k, _)| k == name) {
            Some(slot) => slot.1 = value,
            None => self.runtime.push((name.to_owned(), value)),
        }
    }

    pub fn clear_runtime(&mut self) {
        self.runtime.clear();
    }

    /// `None` runs the collection's requests without an environment.
    pub fn set_active_environment(&mut self, collection: &CollectionName, env: Option<String>) {
        match env {
            Some(env) => self.active_environments.insert(collection.clone(), env),
            None => self.active_environments.remove(collection),
        };
    }

    pub fn active_environment(&self, collection: &CollectionName) -> Option<&Environment> {
        let name = self.active_environments.get(collection)?;
        self.collection(collection)?.environment(name)
    }

    /// The variables request `id` sees, weakest first: globals, its
    /// collection's, its environment's, runtime ones, its own.
    pub fn variables(&self, id: RequestId) -> Variables {
        let found = self.find(id);
        let collection = found.and_then(|f| f.collection);
        let environment = collection.and_then(|c| self.active_environment(&c.name));
        let pairs = |rows: &KeyValues| {
            rows.active()
                .map(|kv| (kv.key.clone(), kv.value.clone()))
                .collect::<Vec<_>>()
        };
        let mut all = pairs(&self.globals);
        if let Some(c) = collection {
            all.extend(pairs(&c.variables));
        }
        if let Some(e) = environment {
            all.extend(pairs(&e.variables));
        }
        all.extend(self.runtime.iter().cloned());
        if let Some(f) = found {
            all.extend(pairs(&f.request.vars));
        }
        Variables::resolve(all.iter().map(|(k, v)| (k.as_str(), v.as_str())))
    }

    /// The variables a collection's settings see: globals, its own, its
    /// active environment's, runtime ones.
    pub fn variables_of_collection(&self, name: &CollectionName) -> Variables {
        let collection = self.collection(name);
        let environment = self.active_environment(name);
        let rows = [
            Some(&self.globals),
            collection.map(|c| &c.variables),
            environment.map(|e| &e.variables),
        ];
        let mut all: Vec<(String, String)> = rows
            .into_iter()
            .flatten()
            .flat_map(|r| r.active().map(|kv| (kv.key.clone(), kv.value.clone())))
            .collect();
        all.extend(self.runtime.iter().cloned());
        Variables::resolve(all.iter().map(|(k, v)| (k.as_str(), v.as_str())))
    }

    fn insert_sorted(&mut self, collection: Collection) {
        let index = self
            .collections
            .partition_point(|c| c.name < collection.name);
        self.collections.insert(index, collection);
    }

    fn free_name(&self, wanted: &CollectionName) -> CollectionName {
        if self.collection(wanted).is_none() {
            return wanted.clone();
        }
        // Bounded: a thousand copies of one name is past any real use.
        (2..=1000)
            .filter_map(|n| CollectionName::parse(&format!("{wanted} ({n})")).ok())
            .find(|candidate| self.collection(candidate).is_none())
            .unwrap_or_else(|| wanted.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_value::KeyValue;

    fn name(s: &str) -> CollectionName {
        CollectionName::parse(s).unwrap()
    }

    fn with_request(collection: &str) -> (Workspace, RequestId) {
        let mut c = Collection::new(name(collection));
        let r = Request::new("r");
        let id = r.id;
        c.items.push(Item::Request(r));
        (Workspace::new(vec![c]), id)
    }

    fn place(collection: &str, folder: Option<RequestId>, index: Option<usize>) -> Place {
        Place {
            collection: name(collection),
            folder,
            index,
        }
    }

    #[test]
    fn editing_a_request_marks_its_collection() {
        let (mut ws, id) = with_request("api");
        assert!(!ws.has_unsaved_changes());
        ws.edit_request(id).unwrap().url = "https://h".into();
        assert!(ws.is_dirty(&name("api")));
        assert_eq!(ws.find(id).unwrap().request.url, "https://h");
        ws.mark_saved(&name("api"));
        assert!(!ws.has_unsaved_changes());
    }

    #[test]
    fn drafts_live_outside_collections() {
        let (mut ws, _) = with_request("api");
        let draft = ws.add_draft(Request::new("Untitled"));
        assert!(ws.find(draft).unwrap().collection.is_none());
        ws.edit_request(draft).unwrap().url = "x".into();
        assert!(!ws.has_unsaved_changes());
        assert!(ws.move_to(draft, &place("api", None, None)));
        assert!(!ws.is_draft(draft));
        assert_eq!(
            ws.find(draft).unwrap().collection.unwrap().name,
            name("api")
        );
    }

    #[test]
    fn moves_between_collections_and_folders() {
        let (mut ws, id) = with_request("a");
        ws.add_collection(name("b")).unwrap();
        let folder = ws.add_folder(&name("b"), None, "users").unwrap();
        assert!(ws.move_to(id, &place("b", Some(folder), None)));
        assert_eq!(ws.collection_of(id).unwrap().name, name("b"));
        assert!(ws.is_dirty(&name("a")) && ws.is_dirty(&name("b")));
        assert!(
            !ws.move_to(folder, &place("b", Some(folder), None)),
            "not into itself"
        );
        assert!(!ws.move_to(id, &place("nope", None, None)));
    }

    #[test]
    fn moving_down_in_the_same_folder() {
        let (mut ws, first) = with_request("a");
        let second = Request::new("second");
        let second_id = second.id;
        ws.edit(&name("a"))
            .unwrap()
            .items
            .push(Item::Request(second));
        // Dropped after the second one (gap index 2).
        assert!(ws.move_to(first, &place("a", None, Some(2))));
        let ids: Vec<_> = ws
            .collection(&name("a"))
            .unwrap()
            .requests()
            .iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(ids, [second_id, first]);
    }

    #[test]
    fn collections_stay_sorted_and_unique() {
        let mut ws = Workspace::new(vec![Collection::new(name("b"))]);
        ws.add_collection(name("a")).unwrap();
        ws.add_collection(name("c")).unwrap();
        assert_eq!(ws.add_collection(name("a")), Err(NameError::Taken));
        let names: Vec<_> = ws.collections().iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["a", "b", "c"]);
    }

    #[test]
    fn imports_get_a_free_name() {
        let mut ws = Workspace::new(vec![Collection::new(name("api"))]);
        assert_eq!(ws.import(Collection::new(name("api"))), name("api (2)"));
        assert_eq!(ws.import(Collection::new(name("api"))), name("api (3)"));
        assert_eq!(ws.import(Collection::new(name("new"))), name("new"));
    }

    #[test]
    fn renaming_keeps_the_requests_and_environment() {
        let (mut ws, id) = with_request("old");
        ws.edit(&name("old"))
            .unwrap()
            .add_environment("dev")
            .unwrap();
        ws.set_active_environment(&name("old"), Some("dev".into()));
        ws.rename_collection(&name("old"), name("new")).unwrap();
        assert_eq!(ws.find(id).unwrap().collection.unwrap().name, name("new"));
        assert!(ws.is_dirty(&name("new")));
        assert_eq!(ws.active_environment(&name("new")).unwrap().name, "dev");
    }

    #[test]
    fn variables_by_precedence() {
        let (mut ws, id) = with_request("api");
        let api = name("api");
        ws.set_globals(KeyValues(vec![
            KeyValue::new("a", "global"),
            KeyValue::new("g", "1"),
        ]));
        let c = ws.edit(&api).unwrap();
        c.variables.0.push(KeyValue::new("a", "collection"));
        c.add_environment("dev").unwrap();
        c.environment_mut("dev")
            .unwrap()
            .variables
            .0
            .push(KeyValue::new("b", "{{a}}-env"));
        ws.set_active_environment(&api, Some("dev".into()));
        let vars = ws.variables(id);
        assert_eq!(
            (vars.get("a"), vars.get("b"), vars.get("g")),
            (Some("collection"), Some("collection-env"), Some("1"))
        );
        ws.set_runtime("a", "runtime".into());
        assert_eq!(ws.variables(id).get("b"), Some("runtime-env"));
        ws.edit_request(id)
            .unwrap()
            .vars
            .0
            .push(KeyValue::new("a", "request"));
        assert_eq!(ws.variables(id).get("a"), Some("request"));
        assert!(!ws.globals_dirty());
        ws.edit_globals().0.clear();
        assert!(ws.has_unsaved_changes());
    }

    #[test]
    fn the_active_environment_must_exist() {
        let (mut ws, _) = with_request("api");
        let api = name("api");
        ws.edit(&api).unwrap().add_environment("dev").unwrap();
        ws.set_active_environment(&api, Some("dev".into()));
        assert_eq!(ws.active_environment(&api).unwrap().name, "dev");
        ws.edit(&api).unwrap().remove_environment("dev");
        assert!(ws.active_environment(&api).is_none());
    }
}
