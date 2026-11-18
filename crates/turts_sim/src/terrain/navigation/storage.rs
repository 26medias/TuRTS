//! Routing buffers use individually reclaimable allocations of at most64KiB.
use std::{
    collections::HashMap,
    hash::{BuildHasherDefault, Hasher},
    mem::size_of,
};
pub(super) const PAGE_BYTES: usize = 64 * 1024;
#[derive(Default)]
pub(super) struct IntegerHasher(u64);
impl Hasher for IntegerHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = self.0.wrapping_mul(0x100000001b3) ^ u64::from(byte);
        }
    }
    fn write_u64(&mut self, value: u64) {
        let mut value = value.wrapping_add(0x9e3779b97f4a7c15);
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        self.0 = value ^ (value >> 31);
    }
    fn write_u32(&mut self, value: u32) {
        self.write_u64(u64::from(value));
    }
}
pub(super) type Index<K, V> = HashMap<K, V, BuildHasherDefault<IntegerHasher>>;
/// Hashbrown's allocated bucket/control capacity, rounded conservatively upward.
/// Inline table headers are counted by their owning struct.
pub(super) fn index_bytes<K, V>(map: &Index<K, V>) -> usize {
    if map.capacity() == 0 {
        return 0;
    }
    map.capacity()
        .next_power_of_two()
        .saturating_mul(size_of::<(K, V)>().saturating_add(1))
        .saturating_add(32)
}
pub(super) fn pages(bytes: usize) -> usize {
    bytes.div_ceil(PAGE_BYTES)
}

pub(super) struct Sparse<T: Copy + Default> {
    directory: Index<u64, usize>,
    buffers: Vec<Vec<T>>,
}
impl<T: Copy + Default> Default for Sparse<T> {
    fn default() -> Self {
        Self {
            directory: Index::default(),
            buffers: Vec::new(),
        }
    }
}
impl<T: Copy + Default> Sparse<T> {
    fn width() -> usize {
        (PAGE_BYTES / size_of::<T>().max(1)).max(1)
    }
    pub(super) fn get(&self, index: u64) -> Option<&T> {
        let width = Self::width() as u64;
        let page = self.directory.get(&(index / width))?;
        self.buffers.get(*page)?.get((index % width) as usize)
    }
    pub(super) fn get_mut(&mut self, index: u64) -> Option<&mut T> {
        let width = Self::width() as u64;
        let page = *self.directory.get(&(index / width))?;
        self.buffers
            .get_mut(page)?
            .get_mut((index % width) as usize)
    }
    pub(super) fn ensure(&mut self, index: u64, limit: usize) -> bool {
        let width = Self::width();
        let key = index / width as u64;
        if self.directory.contains_key(&key) {
            return true;
        }
        // Scratch reservation covers the old directory while a small rehash runs.
        let overhead = (self.directory.capacity().max(4) * 2)
            .saturating_mul(size_of::<(u64, usize)>() + 1)
            .saturating_add((self.buffers.capacity().max(4) * 2) * size_of::<Vec<T>>());
        if self
            .bytes()
            .saturating_add(PAGE_BYTES)
            .saturating_add(overhead)
            > limit
        {
            return false;
        }
        let mut buffer = Vec::new();
        if buffer.try_reserve_exact(width).is_err()
            || buffer.capacity().saturating_mul(size_of::<T>()) > PAGE_BYTES
            || self.directory.try_reserve(1).is_err()
            || self.buffers.try_reserve(1).is_err()
        {
            return false;
        }
        buffer.resize(width, T::default());
        self.directory.insert(key, self.buffers.len());
        self.buffers.push(buffer);
        true
    }
    pub(super) fn bytes(&self) -> usize {
        index_bytes(&self.directory)
            + self.buffers.capacity() * size_of::<Vec<T>>()
            + self
                .buffers
                .iter()
                .map(|buffer| buffer.capacity() * size_of::<T>())
                .sum::<usize>()
    }
    pub(super) fn allocated_pages(&self) -> usize {
        self.buffers.len()
            + pages(index_bytes(&self.directory))
            + pages(self.buffers.capacity() * size_of::<Vec<T>>())
    }
    pub(super) fn retire(&mut self, remaining: &mut usize) {
        while *remaining != 0 && !self.buffers.is_empty() {
            self.buffers.pop();
            *remaining -= 1;
        }
        if self.buffers.is_empty() {
            let directory_pages = pages(index_bytes(&self.directory));
            if directory_pages <= *remaining {
                self.directory = Index::default();
                *remaining -= directory_pages;
            }
            let header_pages = pages(self.buffers.capacity() * size_of::<Vec<T>>());
            if header_pages <= *remaining {
                self.buffers = Vec::new();
                *remaining -= header_pages;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Vertex {
    pub x: u32,
    pub y: u32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct QueueRow {
    pub cost: u64,
    pub vertex: Vertex,
}
#[derive(Default)]
pub(super) struct Frontier {
    buffers: Vec<Vec<QueueRow>>,
    len: usize,
}
impl Frontier {
    fn width() -> usize {
        PAGE_BYTES / size_of::<QueueRow>()
    }
    fn get(&self, index: usize) -> QueueRow {
        self.buffers[index / Self::width()][index % Self::width()]
    }
    fn set(&mut self, index: usize, row: QueueRow) {
        let buffer = &mut self.buffers[index / Self::width()];
        let offset = index % Self::width();
        if offset == buffer.len() {
            buffer.push(row);
        } else {
            buffer[offset] = row;
        }
    }
    pub(super) fn push(&mut self, row: QueueRow, limit: usize) -> bool {
        if self.len / Self::width() == self.buffers.len() {
            if self.bytes().saturating_add(PAGE_BYTES * 2) > limit {
                return false;
            }
            let mut buffer = Vec::new();
            if buffer.try_reserve_exact(Self::width()).is_err()
                || buffer.capacity() * size_of::<QueueRow>() > PAGE_BYTES
                || self.buffers.try_reserve(1).is_err()
            {
                return false;
            }
            self.buffers.push(buffer);
        }
        let mut index = self.len;
        self.len += 1;
        self.set(index, row);
        while index != 0 {
            let parent = (index - 1) / 2;
            let previous = self.get(parent);
            if previous <= row {
                break;
            }
            self.set(index, previous);
            index = parent;
        }
        self.set(index, row);
        true
    }
    pub(super) fn pop(&mut self) -> Option<QueueRow> {
        if self.len == 0 {
            return None;
        }
        let result = self.get(0);
        self.len -= 1;
        if self.len != 0 {
            let row = self.get(self.len);
            let mut index = 0;
            while index * 2 + 1 < self.len {
                let left = index * 2 + 1;
                let right = left + 1;
                let child = if right < self.len && self.get(right) < self.get(left) {
                    right
                } else {
                    left
                };
                let next = self.get(child);
                if row <= next {
                    break;
                }
                self.set(index, next);
                index = child;
            }
            self.set(index, row);
        }
        Some(result)
    }
    pub(super) fn len(&self) -> usize {
        self.len
    }
    pub(super) fn at(&self, index: usize) -> Option<QueueRow> {
        (index < self.len).then(|| self.get(index))
    }
    pub(super) fn clear(&mut self) {
        self.len = 0;
    }
    pub(super) fn bytes(&self) -> usize {
        self.buffers.capacity() * size_of::<Vec<QueueRow>>()
            + self
                .buffers
                .iter()
                .map(|buffer| buffer.capacity() * size_of::<QueueRow>())
                .sum::<usize>()
    }
    pub(super) fn allocated_pages(&self) -> usize {
        self.buffers.len() + pages(self.buffers.capacity() * size_of::<Vec<QueueRow>>())
    }
    pub(super) fn retire(&mut self, remaining: &mut usize) {
        self.len = 0;
        while *remaining != 0 && !self.buffers.is_empty() {
            self.buffers.pop();
            *remaining -= 1;
        }
        if self.buffers.is_empty() {
            let count = pages(self.buffers.capacity() * size_of::<Vec<QueueRow>>());
            if count <= *remaining {
                self.buffers = Vec::new();
                *remaining -= count;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paged_frontier_matches_sorted_oracle_and_retires_within_page_allowance() {
        let mut queue = Frontier::default();
        let mut expected = Vec::new();
        for index in 0..9000u64 {
            let row = QueueRow {
                cost: index.wrapping_mul(7919) % 104729,
                vertex: Vertex {
                    x: index as u32,
                    y: (index % 17) as u32,
                },
            };
            assert!(queue.push(row, 4 * 1024 * 1024));
            expected.push(row);
        }
        expected.sort();
        for row in expected {
            assert_eq!(queue.pop(), Some(row));
        }
        let before = queue.allocated_pages();
        let mut allowance = 1;
        queue.retire(&mut allowance);
        assert_eq!(queue.allocated_pages(), before - 1);
        let mut allowance = 16;
        queue.retire(&mut allowance);
        assert_eq!(queue.bytes(), 0);
    }
}
