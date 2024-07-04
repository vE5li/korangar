#![feature(negative_impls)]
#![feature(auto_traits)]

pub use procedural::RustState;

extern crate self as rust_state;

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

pub trait StateMarker {}

pub auto trait SafeUnwrap {}

pub trait ToUuid {
    fn to_uuid(&self) -> PathUuid;
}

pub trait VecItem {
    type Id: PartialEq + Clone + ToUuid;

    fn get_id(&self) -> Self::Id;
}

pub struct VecLookup<State, Path, Item>
where
    Item: VecItem,
{
    path: Path,
    id: Item::Id,
    _marker: PhantomData<State>,
}

impl<State, Path, Item> VecLookup<State, Path, Item>
where
    Item: VecItem,
{
    pub fn new(path: Path, id: Item::Id) -> Self {
        Self {
            path,
            id,
            _marker: PhantomData,
        }
    }
}

impl<State, Path, Item> Clone for VecLookup<State, Path, Item>
where
    Path: Clone,
    Item: VecItem,
{
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            id: self.id.clone(),
            _marker: PhantomData,
        }
    }
}

impl<'a, State, Path, Item> Selector<'a, State, Item> for VecLookup<State, Path, Item>
where
    State: StateMarker + 'static,
    Path: Selector<'a, State, Vec<Item>>,
    Item: VecItem + 'static,
{
    fn get(&self, state: &'a State) -> Option<&'a Item> {
        self.path.get(state)?.iter().find(|e| e.get_id() == self.id)
    }

    fn get_mut(&self, state: &'a mut State) -> Option<&'a mut Item> {
        self.path.get_mut(state)?.iter_mut().find(|e| e.get_id() == self.id)
    }

    fn get_path_id(&self) -> PathId {
        let mut inner = self.path.get_path_id();
        inner.parts.push(self.id.to_uuid());
        inner
    }
}

impl<State, Path, Item> !SafeUnwrap for VecLookup<State, Path, Item> {}

pub trait MapItem {
    type Id: Eq + PartialEq + Hash + Clone + ToUuid;
}

pub struct MapLookup<State, Path, Item>
where
    Item: MapItem,
{
    path: Path,
    id: Item::Id,
    _marker: PhantomData<State>,
}

impl<State, Path, Item> MapLookup<State, Path, Item>
where
    Item: MapItem,
{
    pub fn new(path: Path, id: Item::Id) -> Self {
        Self {
            path,
            id,
            _marker: PhantomData,
        }
    }
}

impl<State, Path, Item> Clone for MapLookup<State, Path, Item>
where
    Path: Clone,
    Item: MapItem,
{
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            id: self.id.clone(),
            _marker: PhantomData,
        }
    }
}

impl<'a, State, Path, Item> Selector<'a, State, Item> for MapLookup<State, Path, Item>
where
    State: StateMarker + 'static,
    Path: Selector<'a, State, HashMap<Item::Id, Item>>,
    Item: MapItem + 'static,
{
    fn get(&self, state: &'a State) -> Option<&'a Item> {
        self.path.get(state)?.get(&self.id)
    }

    fn get_mut(&self, state: &'a mut State) -> Option<&'a mut Item> {
        self.path.get_mut(state)?.get_mut(&self.id)
    }

    fn get_path_id(&self) -> PathId {
        let mut inner = self.path.get_path_id();
        inner.parts.push(self.id.to_uuid());
        inner
    }
}

impl<State, Path, Item> !SafeUnwrap for MapLookup<State, Path, Item> {}

pub trait Selector<'a, State, To>: Clone + 'static {
    fn get(&self, state: &'a State) -> Option<&'a To>;

    fn get_mut(&self, state: &'a mut State) -> Option<&'a mut To>;

    fn get_path_id(&self) -> PathId;
}

pub type StateChange<State> = Box<dyn FnOnce(&mut State)>;

pub struct Context<State> {
    state: State,
    state_changes: UnsafeCell<Vec<StateChange<State>>>,
    updated_paths: UnsafeCell<Vec<PathId>>,
    change_map: ChangeMap,
    version: u32,
}

impl<State: StateMarker> Context<State> {
    pub fn new(state: State) -> Self {
        Self {
            state,
            state_changes: UnsafeCell::new(Vec::new()),
            updated_paths: UnsafeCell::new(Vec::new()),
            change_map: ChangeMap::default(),
            version: 0,
        }
    }

    fn push_change(&self, path_id: PathId, state_change: StateChange<State>) {
        let updated_paths = UnsafeCell::raw_get(&self.updated_paths as *const UnsafeCell<Vec<PathId>>);
        let updated_paths = unsafe { &mut *updated_paths };
        updated_paths.push(path_id);

        let state_changes = UnsafeCell::raw_get(&self.state_changes as *const UnsafeCell<Vec<StateChange<State>>>);
        let state_changes = unsafe { &mut *state_changes };
        state_changes.push(state_change);
    }

    pub fn update_value<Path, Value>(&self, path: &Path, value: Value)
    where
        Path: for<'a> Selector<'a, State, Value>,
        Value: 'static,
    {
        let path = path.clone();
        self.push_change(
            path.get_path_id(),
            Box::new(move |state: &mut State| match path.get_mut(state) {
                Some(reference) => *reference = value,
                None => println!("Failed to update state"),
            }),
        );
    }

    pub fn vec_push<Path, Value>(&self, path: &Path, value: Value)
    where
        Path: for<'a> Selector<'a, State, Vec<Value>>,
        Value: 'static,
    {
        let path = path.clone();
        self.push_change(
            path.get_path_id(),
            Box::new(move |state: &mut State| match path.get_mut(state) {
                Some(reference) => reference.push(value),
                None => println!("Failed to update state"),
            }),
        );
    }

    pub fn vec_remove<Path, Value>(&self, path: &Path, id: Value::Id)
    where
        Path: for<'a> Selector<'a, State, Vec<Value>>,
        Value: VecItem + 'static,
    {
        let path = path.clone();
        self.push_change(
            path.get_path_id(),
            Box::new(move |state: &mut State| match path.get_mut(state) {
                Some(reference) => reference.retain(|item| item.get_id() != id),
                None => println!("Failed to update state"),
            }),
        );
    }

    pub fn map_insert<Path, Value>(&self, path: &Path, id: Value::Id, value: Value)
    where
        Path: for<'a> Selector<'a, State, HashMap<Value::Id, Value>>,
        Value: MapItem + 'static,
    {
        let path = path.clone();
        self.push_change(
            path.get_path_id(),
            Box::new(move |state: &mut State| match path.get_mut(state) {
                Some(reference) => {
                    reference.insert(id, value);
                }
                None => println!("Failed to update state"),
            }),
        );
    }

    pub fn map_remove<Path, Value>(&self, path: &Path, id: Value::Id)
    where
        Path: for<'a> Selector<'a, State, HashMap<Value::Id, Value>>,
        Value: MapItem + 'static,
    {
        let path = path.clone();
        self.push_change(
            path.get_path_id(),
            Box::new(move |state: &mut State| match path.get_mut(state) {
                Some(reference) => {
                    reference.remove(&id);
                }
                None => println!("Failed to update state"),
            }),
        );
    }

    pub fn apply(&mut self) {
        self.version += 1;

        self.state_changes.get_mut().drain(..).for_each(|apply| apply(&mut self.state));

        self.updated_paths
            .get_mut()
            .drain(..)
            .for_each(|path| self.change_map.update_path(path, self.version));
    }

    pub fn get<'a, Path, Output>(&'a self, path: &Path) -> Option<&'a Output>
    where
        Path: Selector<'a, State, Output>,
    {
        path.get(&self.state)
    }

    pub fn get_safe<'a, Path, Output>(&'a self, path: &Path) -> &'a Output
    where
        Path: Selector<'a, State, Output> + SafeUnwrap,
    {
        path.get(&self.state).unwrap()
    }

    pub fn get_version(&self) -> u32 {
        self.version
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PathUuid(pub u32);

#[derive(Debug)]
pub struct PathId {
    parts: Vec<PathUuid>,
}

impl PathId {
    pub fn new(parts: Vec<PathUuid>) -> Self {
        Self { parts }
    }

    pub fn push(&mut self, part: PathUuid) {
        self.parts.push(part);
    }

    pub fn contains_subpath(&self, other: &PathId) -> bool {
        fn compare_slice(path: &[PathUuid], subpath: &[PathUuid]) -> bool {
            match (path, subpath) {
                // If either of the slices is empty, one was more generic than the other. If the
                // earlier parts of the path didn't match, we would not have gotten here, so the
                // paths must match.
                ([], _remaining) | (_remaining, []) => true,
                ([left, ..], [right, ..]) => match *left == *right {
                    true => compare_slice(&path[1..], &subpath[1..]),
                    false => false,
                },
            }
        }

        compare_slice(&self.parts[..], &other.parts[..])
    }
}

#[derive(Debug)]
pub enum ChangeEntry {
    Leaf(u32),
    Complex(ChangeMap),
}

#[derive(Debug, Default)]
pub struct ChangeMap {
    map: HashMap<PathUuid, ChangeEntry>,
    version: u32,
}

impl ChangeMap {
    fn update_all(&mut self, version: u32) {
        self.version = version;

        self.map.values_mut().for_each(|entry| match entry {
            ChangeEntry::Leaf(old_version) => *old_version = version,
            ChangeEntry::Complex(map) => map.update_all(version),
        })
    }

    fn update_path_inner(&mut self, parts: &[PathUuid], version: u32) {
        self.version = version;

        match parts {
            [uuid] => {
                let Some(entry) = self.map.get_mut(uuid) else {
                    self.map.insert(*uuid, ChangeEntry::Leaf(version));
                    return;
                };

                match entry {
                    ChangeEntry::Complex(inner_map) => inner_map.update_all(version),
                    ChangeEntry::Leaf(old_version) => *old_version = version,
                }
            }
            [uuid, ..] => {
                let Some(entry) = self.map.get_mut(uuid) else {
                    let mut change_map = ChangeMap::default();
                    change_map.update_path_inner(&parts[1..], version);

                    self.map.insert(*uuid, ChangeEntry::Complex(change_map));
                    return;
                };

                match entry {
                    ChangeEntry::Complex(inner_map) => inner_map.update_path_inner(&parts[1..], version),
                    _ => {
                        let mut change_map = ChangeMap::default();
                        change_map.update_path_inner(&parts[1..], version);

                        self.map.insert(*uuid, ChangeEntry::Complex(change_map));
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn update_path(&mut self, path_id: PathId, version: u32) {
        self.update_path_inner(&path_id.parts, version);
    }

    fn is_path_out_of_date_inner(&self, parts: &[PathUuid], version: u32) -> bool {
        match parts {
            [uuid] => {
                let Some(entry) = self.map.get(uuid) else {
                    return false;
                };

                match entry {
                    ChangeEntry::Complex(inner_map) => inner_map.version > version,
                    ChangeEntry::Leaf(old_version) => *old_version > version,
                }
            }
            [uuid, ..] => {
                let Some(entry) = self.map.get(uuid) else {
                    return false;
                };

                match entry {
                    ChangeEntry::Complex(inner_map) => inner_map.is_path_out_of_date_inner(&parts[1..], version),
                    ChangeEntry::Leaf(..) => true,
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn is_path_out_of_date(&self, path_read: &PathRead) -> bool {
        self.is_path_out_of_date_inner(&path_read.path.parts, path_read.version)
    }
}

#[derive(Debug)]
pub struct PathRead {
    path: PathId,
    version: u32,
}

#[derive(Debug, Default)]
pub struct ReadState {
    accesses: Vec<PathRead>,
}

impl ReadState {
    pub fn track_new<'s, State>(&'s mut self, context: &'s mut Context<State>) -> Tracker<'s, State>
    where
        State: StateMarker,
    {
        self.accesses.clear();
        self.track(context)
    }

    pub fn track<'s, State>(&'s mut self, context: &'s mut Context<State>) -> Tracker<'s, State>
    where
        State: StateMarker,
    {
        Tracker {
            context,
            accesses: UnsafeCell::new(&mut self.accesses),
        }
    }

    pub fn is_out_of_date<State>(&self, context: &Context<State>) -> bool
    where
        State: StateMarker,
    {
        self.accesses
            .iter()
            .any(|path_read| context.change_map.is_path_out_of_date(path_read))
    }
}

pub struct Tracker<'s, State> {
    context: &'s Context<State>,
    accesses: UnsafeCell<&'s mut Vec<PathRead>>,
}

impl<'s, State> Tracker<'s, State>
where
    State: StateMarker,
{
    fn register_read(&self, path: PathId) {
        let accesses = UnsafeCell::raw_get(&self.accesses as *const UnsafeCell<&'s mut Vec<PathRead>>);
        let accesses = unsafe { &mut *accesses };
        accesses.push(PathRead {
            path,
            version: self.context.get_version(),
        });
    }

    pub fn get<'a, Path, Output>(&'a self, path: &Path) -> Option<&'a Output>
    where
        Path: Selector<'a, State, Output>,
    {
        self.register_read(path.get_path_id());
        self.context.get(path)
    }

    pub fn get_safe<'a, Path, Output>(&'a self, path: &Path) -> &'a Output
    where
        Path: Selector<'a, State, Output> + SafeUnwrap,
    {
        self.register_read(path.get_path_id());
        self.context.get_safe(path)
    }
}

#[derive(RustState)]
struct State<T>
where
    T: Clone + 'static,
{
    other: String,
    pd: std::marker::PhantomData<T>,
}

#[cfg(test)]
mod test {
    use procedural::RustState;

    use crate::{Context, PathUuid, Selector, ToUuid, VecItem, VecLookup};

    impl ToUuid for u32 {
        fn to_uuid(&self) -> PathUuid {
            PathUuid(*self)
        }
    }

    impl VecItem for Entity {
        type Id = u32;

        fn get_id(&self) -> u32 {
            self.id
        }
    }

    #[derive(Debug, RustState)]
    struct Entity {
        id: u32,
    }

    struct Using<T>
    where
        T: for<'a> Selector<'a, State, u32>,
    {
        path: T,
    }

    impl<T> Using<T>
    where
        T: for<'a> Selector<'a, State, u32>,
    {
        fn use_(&self, state: &Context<State>) {
            println!("{:?}", state.get(&self.path));
        }
    }

    #[derive(RustState)]
    #[state_root]
    struct State {
        entities: Vec<Entity>,
        other: String,
    }

    fn update_interface(state: &mut Context<State>) {
        println!("Updating UI with: {:?}", state.get(&State::entities()));

        state.remove(&State::entities(), 23);
        state.push(&State::entities(), Entity { id: 50 });
    }

    fn initialize_state() -> Context<State> {
        Context::new(State {
            entities: vec![Entity { id: 23 }],
            other: String::from("Before"),
        })
    }

    #[test]
    fn test() {
        let mut context = initialize_state();

        let path = Entity::id(VecLookup::new(State::entities(), 23));

        println!("{:?}", context.get(&path));

        let path = Entity::id(VecLookup::new(State::entities(), 26));
        Using { path }.use_(&context);

        update_interface(&mut context);

        let context = context.apply();

        let path = Entity::id(VecLookup::new(State::entities(), 50));
        println!("After UI: {:?}", context.get(&path));
        println!("PathId: {:?}", path.get_path_id());

        let en_12_id = Entity::id(VecLookup::new(State::entities(), 23)).get_path_id();
        let en_12 = VecLookup::new(State::entities(), 23).get_path_id();

        assert!(en_12_id.contains_subpath(&en_12));
        assert!(en_12.contains_subpath(&en_12_id));
    }

    #[test]
    fn index_present() {
        let context = initialize_state();
        let path = Entity::id(VecLookup::new(State::entities(), 23));
        assert_eq!(context.get(&path), Some(&23));
    }

    #[test]
    fn index_not_present() {
        let context = initialize_state();
        let path = Entity::id(VecLookup::new(State::entities(), 40));
        assert_eq!(context.get(&path), None);
    }

    #[test]
    fn apply_changes() {
        let mut context = initialize_state();
        let path = State::other();
        assert_eq!(context.get_safe(&path).as_str(), "Before");

        context.update_value(&path, String::from("After"));
        assert_eq!(context.get_safe(&path).as_str(), "Before");

        let context = context.apply();
        assert_eq!(context.get_safe(&path).as_str(), "After");
    }

    // #[test]
    // fn composite() {
    //     let mut context = initialize_state();
    //     let path = Entity::ree(State::other());
    //
    //     assert_eq!(*context.get_safe(&path), 10.0);
    // }
}
