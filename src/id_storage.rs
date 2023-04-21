use std::{mem::MaybeUninit, iter::{Enumerate, FilterMap}};

use crate::{StandardVersionedIndexId, VersionedIndexId};

pub struct IdStorage<Id: VersionedIndexId = StandardVersionedIndexId> {
    ids: Vec<Id>,
    free_list_head: usize,
    free_list_size: usize,
}

impl<Id: VersionedIndexId> IdStorage<Id> {
    const FREE_LIST_END: usize = Id::MAX_INDEX;
    pub const MAX_SIZE: usize = Id::MAX_INDEX;

    pub fn new() -> Self {
        Self {
            ids: vec![],
            free_list_head: Self::FREE_LIST_END,
            free_list_size: 0,
        }
    }

    pub fn len(&self) -> usize {
        return self.ids.len() - self.free_list_size;
    }

    pub fn reserve(&mut self) -> Id {
        if self.free_list_head != Self::FREE_LIST_END {
            let index = self.free_list_head;
            let indexed_id = self.ids[index];
            self.free_list_head = indexed_id.index();

            let id = Id::from_index_and_version(index, indexed_id.version()).next_version_id();
            self.ids[index] = id;
            self.free_list_size -= 1;

            return id;
        } else {
            let id = Id::from_index(self.ids.len().into());
            self.ids.push(id);
            return id;
        }
    }

    pub fn free(&mut self, id: Id) {
        assert!(self.contains(id));
        let index = id.index();
        self.ids[index] = Id::from_index_and_version(self.free_list_head, id.version());
        self.free_list_head = id.index();
        self.free_list_size += 1;
    }

    pub fn contains(&self, id: Id) -> bool {
        return id.index() < self.ids.len() && self.ids[id.index()] == id;
    }
}

fn id_filter<Id: VersionedIndexId>(p: (usize, &Id)) -> Option<Id> {
    if p.0 == p.1.index() {
        return Some(*p.1);
    } else {
        return None;
    }
}

impl<'a, Id: VersionedIndexId> IntoIterator for &'a IdStorage<Id> {
    type Item = Id;
    type IntoIter = FilterMap<Enumerate<std::slice::Iter<'a, Id>>, fn((usize, &'a Id)) -> Option<Id>>;

    fn into_iter(self) -> Self::IntoIter {
        self.ids.iter().enumerate().filter_map(id_filter)
    }
}

#[test]
fn id_storage_works() {
    type Id = StandardVersionedIndexId;
    let mut storage = IdStorage::<Id>::new();
    assert_eq!(storage.len(), 0);
    assert_eq!(storage.into_iter().collect::<Vec<_>>(), vec![]);

    let id = storage.reserve();
    assert_eq!(storage.len(), 1);
    assert!(storage.contains(id));
    assert_eq!(storage.into_iter().collect::<Vec<_>>(), vec![id]);

    storage.free(id);
    assert_eq!(storage.len(), 0);
    assert!(!storage.contains(id));
    assert_eq!(storage.into_iter().collect::<Vec<_>>(), vec![]);

    let second_id = storage.reserve();
    assert_eq!(storage.len(), 1);
    assert!(!storage.contains(id));
    assert!(storage.contains(second_id));
    assert_eq!(storage.into_iter().collect::<Vec<_>>(), vec![second_id]);
}

pub struct IdMap<Id: VersionedIndexId, T> {
    ids: IdStorage<Id>,
    values: Vec<MaybeUninit<T>>,
}

impl<Id: VersionedIndexId, T> IdMap<Id, T> {
    pub fn new() -> Self {
        Self {
            ids: IdStorage::new(),
            values: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        return self.ids.len();
    }

    pub fn insert(&mut self, value: T) -> (Id, &mut T) {
        let id = self.ids.reserve();
        if id.index() >= self.values.len() {
            self.values.resize_with(self.ids.len(), || MaybeUninit::uninit());
        }
        return (id, self.values[id.index()].write(value));
    }

    pub fn get(&self, id: Id) -> Option<&T> {
        if self.ids.contains(id) {
            unsafe {
                return Some(self.values[id.index()].assume_init_ref());
            }
        } else {
            return None;
        }
    }

    pub fn get_mut(&mut self, id: Id) -> Option<&mut T> {
        if self.ids.contains(id) {
            unsafe {
                return Some(self.values[id.index()].assume_init_mut());
            }
        } else {
            return None;
        }
    }

    pub fn remove(&mut self, id: Id) -> T {
        assert!(self.contains(id));
        self.ids.free(id);
        unsafe {
            return self.values[id.index()].assume_init_read();
        }
    }

    pub fn contains(&self, id: Id) -> bool {
        return self.ids.contains(id);
    }
}

impl<Id: VersionedIndexId, T> Drop for IdMap<Id, T> {
    fn drop(&mut self) {
        for id in &self.ids {
            unsafe {
                self.values[id.index()].assume_init_drop();
            }
        }
    }
}

pub struct IdMapIntoIterator<'a, Id: VersionedIndexId, T> {
    id_iterator: <&'a IdStorage<Id> as IntoIterator>::IntoIter,
    values: &'a [MaybeUninit<T>],
}

impl<'a, Id: VersionedIndexId, T> Iterator for IdMapIntoIterator<'a, Id, T> {
    type Item = (Id, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(id) = self.id_iterator.next() {
            unsafe {
                return Some((id, self.values[id.index()].assume_init_ref()));
            }
        } else {
            return None
        }
    }
}

pub struct IdMapMutIntoIterator<'a, Id: VersionedIndexId, T> {
    id_iterator: <&'a IdStorage<Id> as IntoIterator>::IntoIter,
    values: &'a mut [MaybeUninit<T>],
}

impl<'a, Id: VersionedIndexId, T> Iterator for IdMapMutIntoIterator<'a, Id, T> {
    type Item = (Id, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(id) = self.id_iterator.next() {
            unsafe {
                return Some((id, self.values[id.index()].as_mut_ptr().as_mut().unwrap_unchecked())); // TODO: is this safe? :D
            }
        } else {
            return None
        }
    }
}

impl<'a, Id: VersionedIndexId, T> IntoIterator for &'a mut IdMap<Id, T> {
    type Item = (Id, &'a mut T);
    type IntoIter = IdMapMutIntoIterator<'a, Id, T>;

    fn into_iter(self) -> Self::IntoIter {
        return Self::IntoIter {
            id_iterator: self.ids.into_iter(),
            values: &mut self.values,
        };
    }
}

impl<'a, Id: VersionedIndexId, T> IntoIterator for &'a IdMap<Id, T> {
    type Item = (Id, &'a T);
    type IntoIter = IdMapIntoIterator<'a, Id, T>;

    fn into_iter(self) -> Self::IntoIter {
        return Self::IntoIter {
            id_iterator: self.ids.into_iter(),
            values: &self.values,
        };
    }
}


mod test {
    use std::rc::Rc;
    use super::*;

    #[test]
    fn id_map_works() {
        type Id = StandardVersionedIndexId;
        type T = Rc<i32>;

        let mut map = IdMap::<Id, T>::new();
        assert_eq!(map.len(), 0);
        assert_eq!(map.into_iter().collect::<Vec<_>>(), []);

        let (id, value) = map.insert(T::new(42));
        assert_eq!(Rc::strong_count(value), 1);
        assert!(map.contains(id));
        assert!(map.get(id).is_some());
        assert_eq!(map.len(), 1);

        let value = map.get(id).expect("should be there").clone();
        assert_eq!(Rc::strong_count(&value), 2);


        {
            let values = map.into_iter().collect::<Vec<_>>();
            assert_eq!(values.len(), 1);
            assert_eq!(values[0].0, id);
            assert_eq!(*values[0].1.clone(), 42);
        }

        map.remove(id);
        assert_eq!(map.len(), 0);
        assert!(map.get(id).is_none());
        assert_eq!(Rc::strong_count(&value), 1);
    }
}

pub struct SimpleStorage<T> {
    data: Option<T>,
}

impl<T> SimpleStorage<T> {
    pub fn new() -> Self {
        Self { data: None }
    }

    pub fn emplace(&mut self, value: T) -> &T {
        self.data = Some(value);
        unsafe {
            return self.data.as_ref().unwrap_unchecked();
        }
    }

    pub fn reset(&mut self) {
        self.data = None;
    }

    pub fn get(&mut self) -> Option<&T> {
        self.data.as_ref()
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.data.as_mut()
    }
}
