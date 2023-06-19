use std::{mem::size_of, fmt::Display, hash::Hash};

// #[derive(Debug, Copy, Clone, PartialEq, Eq)]
// pub struct VersionedIndexId<const VERSION_BITS: usize> {
//     id: i32,
// }

// impl<const VERSION_BITS: usize> VersionedIndexId<VERSION_BITS> {
//     pub const INDEX_BITS: usize = size_of::<i32>() * 8 - VERSION_BITS;
//     pub const VERSION_BITS: usize = VERSION_BITS;
//     pub const NUM_INDICES: usize = 1 << Self::INDEX_BITS;
//     pub const MAX_INDEX: usize = Self::NUM_INDICES - 1;
//     pub const NUM_VERSIONS: usize = 1 << Self::VERSION_BITS;
//     pub const MAX_VERSION: usize = Self::NUM_VERSIONS - 1;

//     pub fn from_id(id: usize) -> Self { Self { id: id.try_into().unwrap() } }

//     pub fn from_index(index: usize) -> Self {
//         assert!(index < (1 << Self::INDEX_BITS));
//         return Self {
//             id: index.try_into().unwrap(),
//         }
//     }

//     pub fn from_index_and_version(index: usize, version: usize) -> Self {
//         assert!(index < (1 << Self::INDEX_BITS));
//         assert!(version < (1 << VERSION_BITS));
//         return Self {
//             id: (index + (version << Self::INDEX_BITS)).try_into().unwrap(),
//         }
//     }

//     pub fn id(&self) -> i32 { self.id }
//     pub fn version(&self) -> usize { (self.id >> Self::INDEX_BITS).try_into().unwrap() }
//     pub fn index(&self) -> usize { <i32 as TryInto<usize>>::try_into(self.id).unwrap() & Self::MAX_INDEX }

//     pub fn next_version_id(&self) -> Self {
//         return Self::from_index_and_version(self.index(), (self.version() + 1) % Self::NUM_VERSIONS);
//     }
// }

// #[test]
// fn versioned_index_id_works() {
//     type Id = VersionedIndexId<8>;
//     let id = Id::from_index(10);
//     assert_eq!(id.index(), 10);
//     assert_eq!(id.version(), 0);

//     let next_id = id.next_version_id();
// }

pub trait VersionedIndexId: Send + Sync + Copy + Eq + Display + Hash {
    const INDEX_BITS: usize;
    const VERSION_BITS: usize;
    const NUM_INDICES: usize;
    const MAX_INDEX: usize;
    const NUM_VERSIONS: usize;
    const MAX_VERSION: usize;

    fn from_index(index: usize) -> Self;
    fn from_index_and_version(index: usize, version: usize) -> Self;

    fn id(&self) -> i32;
    fn version(&self) -> usize;
    fn index(&self) -> usize;

    fn next_version_id(&self) -> Self;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct StandardVersionedIndexId<const VERSION_BITS: usize = 8> {
    id: u32,
}

impl<const VERSION_BITS: usize> StandardVersionedIndexId<VERSION_BITS> {
    pub const fn from_index_and_version(index: u32, version: u32) -> Self {
        assert!(index < (1u32 << Self::INDEX_BITS));
        assert!(version < (1u32 << Self::VERSION_BITS));
        return Self {
            id: index + (version << Self::INDEX_BITS),
        }
    }
}

impl<const VERSION_BITS: usize> VersionedIndexId for StandardVersionedIndexId<VERSION_BITS> {
    const INDEX_BITS: usize = size_of::<i32>() * 8 - VERSION_BITS;
    const VERSION_BITS: usize = VERSION_BITS;
    const NUM_INDICES: usize = 1 << Self::INDEX_BITS;
    const MAX_INDEX: usize = Self::NUM_INDICES - 1;
    const NUM_VERSIONS: usize = 1 << Self::VERSION_BITS;
    const MAX_VERSION: usize = Self::NUM_VERSIONS - 1;

    fn from_index(index: usize) -> Self {
        assert!(index < (1 << Self::INDEX_BITS));
        return Self {
            id: index.try_into().unwrap(),
        }
    }

    fn from_index_and_version(index: usize, version: usize) -> Self {
        assert!(index < (1usize << Self::INDEX_BITS));
        assert!(version < (1usize << Self::VERSION_BITS));
        return Self {
            id: (index + (version << Self::INDEX_BITS)).try_into().unwrap(),
        }
    }

    fn id(&self) -> i32 { i32::from_ne_bytes(self.id.to_ne_bytes()) }
    fn version(&self) -> usize { (self.id >> Self::INDEX_BITS).try_into().unwrap() }
    fn index(&self) -> usize { <u32 as TryInto<usize>>::try_into(self.id).unwrap() & Self::MAX_INDEX }

    fn next_version_id(&self) -> Self {
        return VersionedIndexId::from_index_and_version(self.index(), (self.version() + 1) & Self::MAX_VERSION);
    }
}


impl<const VERSION_BITS: usize> Display for StandardVersionedIndexId<VERSION_BITS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{};{}]", self.index(), self.version())
    }
}

#[test]
fn versioned_index_id_works() {
    type Id = StandardVersionedIndexId<8>;
    let id = Id::from_index(10);
    assert_eq!(id.index(), 10);
    assert_eq!(id.version(), 0);

    let next_id = id.next_version_id();
    assert_eq!(next_id.index(), 10);
    assert_eq!(next_id.version(), 1);

    let wrapped_around = Id::from_index_and_version(23, 255).next_version_id();
    assert_eq!(wrapped_around.index(), 23);
    assert_eq!(wrapped_around.version(), 0);
}
