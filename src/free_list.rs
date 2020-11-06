use std::default::Default;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FreeListHandle(pub(crate) u32);

impl FreeListHandle {
    pub const NONE: Self = FreeListHandle(std::u32::MAX);

    pub fn is_none(self) -> bool { self == Self::NONE }

    pub fn is_some(self) -> bool { self != Self::NONE }

    fn to_usize(self) -> usize { self.0 as usize }

    pub fn serialize(&self) -> u32 { self.0 }

    pub fn deserialize(bytes: u32) -> Self { FreeListHandle(bytes) }
}

pub struct FreeListItem<T> {
    item: T,
    next: FreeListHandle,
}

pub struct FreeList<T> {
    items: Vec<FreeListItem<T>>,
    first_free: FreeListHandle,
}

impl<T> FreeList<T> {
    pub fn new() -> Self {
        FreeList {
            items: Vec::new(),
            first_free: FreeListHandle::NONE,
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        FreeList {
            items: Vec::with_capacity(cap),
            first_free: FreeListHandle::NONE,
        }
    }

    pub fn add_with_value(&mut self, val: T) -> FreeListHandle {
        if self.first_free.is_some() {
            let idx = self.first_free;
            self.first_free = self.items[idx.to_usize()].next;
            self.items[idx.to_usize()].item = val;
            self.items[idx.to_usize()].next = FreeListHandle::NONE;

            return idx;
        }

        let idx = FreeListHandle(self.items.len() as u32);
        self.items.push(FreeListItem {
            item: val,
            next: FreeListHandle::NONE,
        });

        idx
    }
 
    pub fn add(&mut self) -> FreeListHandle
        where T: Default
    {
        if self.first_free.is_some() {
            let idx = self.first_free;
            self.first_free = self.items[idx.to_usize()].next;
            self.items[idx.to_usize()].next = FreeListHandle::NONE;

            return idx;
        }

        let idx = FreeListHandle(self.items.len() as u32);
        self.items.push(FreeListItem {
            item: Default::default(),
            next: FreeListHandle::NONE,
        });

        idx
    }

    pub fn remove(&mut self, handle: FreeListHandle) {
        self.items[handle.to_usize()].next = self.first_free;
        self.first_free = handle;
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            items: self.items.iter(),
        }
    }

    pub fn iter_with_handles(&self) -> IterWithHandles<T> {
        IterWithHandles {
            items: self.items.iter(),
            current: FreeListHandle(0),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            items: self.items.iter_mut(),
        }
    }

    pub fn iter_mut_with_handles(&mut self) -> IterMutWithHandles<T> {
        IterMutWithHandles {
            items: self.items.iter_mut(),
            current: FreeListHandle(0),
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.first_free = FreeListHandle::NONE;
    }
}

pub struct Iter<'l, T> {
    items: std::slice::Iter<'l, FreeListItem<T>>,
}

impl<'l, T> Iterator for Iter<'l, T> {
    type Item = &'l T;
    fn next(&mut self) -> Option<&'l T> {
        while let Some(item) = self.items.next() {
            if item.next.is_none() {
                return Some(&item.item);
            }
        }

        None
    }
}

pub struct IterWithHandles<'l, T> {
    items: std::slice::Iter<'l, FreeListItem<T>>,
    current: FreeListHandle,
}

impl<'l, T> Iterator for IterWithHandles<'l, T> {
    type Item = (FreeListHandle, &'l T);
    fn next(&mut self) -> Option<(FreeListHandle, &'l T)> {
        while let Some(item) = self.items.next() {
            let handle = self.current;
            self.current.0 += 1;
            if item.next.is_none() {
                return Some((handle, &item.item));
            }
        }

        None
    }
}

pub struct IterMut<'l, T> {
    items: std::slice::IterMut<'l, FreeListItem<T>>,
}

impl<'l, T> Iterator for IterMut<'l, T> {
    type Item = &'l mut T;
    fn next(&mut self) -> Option<&'l mut T> {
        while let Some(item) = self.items.next() {
            if item.next.is_none() {
                return Some(&mut item.item);
            }
        }

        None
    }
}

pub struct IterMutWithHandles<'l, T> {
    items: std::slice::IterMut<'l, FreeListItem<T>>,
    current: FreeListHandle,
}

impl<'l, T> Iterator for IterMutWithHandles<'l, T> {
    type Item = (FreeListHandle, &'l mut T);
    fn next(&mut self) -> Option<(FreeListHandle, &'l mut T)> {
        while let Some(item) = self.items.next() {
            let handle = self.current;
            self.current.0 += 1;
            if item.next.is_none() {
                return Some((handle, &mut item.item));
            }
        }

        None
    }
}

impl<T> std::ops::Index<FreeListHandle> for FreeList<T> {
    type Output = T;
    fn index(&self, id: FreeListHandle) -> &T {
        &self.items[id.to_usize()].item
    }
}

impl<T> std::ops::IndexMut<FreeListHandle> for FreeList<T> {
    fn index_mut(&mut self, id: FreeListHandle) -> &mut T {
        &mut self.items[id.to_usize()].item
    }
}

impl<'l, T> IntoIterator for &'l FreeList<T> {
    type Item = &'l T;
    type IntoIter = Iter<'l, T>;

    fn into_iter(self) -> Iter<'l, T> { self.iter() }
}

impl<'l, T> IntoIterator for &'l mut FreeList<T> {
    type Item = &'l mut T;
    type IntoIter = IterMut<'l, T>;

    fn into_iter(self) -> IterMut<'l, T> { self.iter_mut() }
}

