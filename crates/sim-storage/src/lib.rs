#![forbid(unsafe_code)]

#[derive(Clone, Debug, PartialEq)]
pub struct Dense1<T> {
    values: Vec<T>,
}

impl<T: Clone> Dense1<T> {
    pub fn new(len: usize, value: T) -> Self {
        Self {
            values: vec![value; len],
        }
    }
}

impl<T> Dense1<T> {
    pub fn from_vec(values: Vec<T>) -> Self {
        Self { values }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.values.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.values.get_mut(index)
    }

    pub fn as_slice(&self) -> &[T] {
        &self.values
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.values
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dense2<T> {
    rows: usize,
    cols: usize,
    values: Vec<T>,
}

impl<T: Clone> Dense2<T> {
    pub fn new(rows: usize, cols: usize, value: T) -> Self {
        Self {
            rows,
            cols,
            values: vec![value; rows * cols],
        }
    }
}

impl<T> Dense2<T> {
    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn get(&self, row: usize, col: usize) -> Option<&T> {
        if row >= self.rows || col >= self.cols {
            return None;
        }
        self.values.get(row * self.cols + col)
    }

    pub fn get_mut(&mut self, row: usize, col: usize) -> Option<&mut T> {
        if row >= self.rows || col >= self.cols {
            return None;
        }
        self.values.get_mut(row * self.cols + col)
    }

    pub fn row(&self, row: usize) -> Option<&[T]> {
        if row >= self.rows {
            return None;
        }
        let start = row * self.cols;
        Some(&self.values[start..start + self.cols])
    }

    pub fn row_mut(&mut self, row: usize) -> Option<&mut [T]> {
        if row >= self.rows {
            return None;
        }
        let start = row * self.cols;
        Some(&mut self.values[start..start + self.cols])
    }

    pub fn canonical_values(&self) -> &[T] {
        &self.values
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StableHandle(pub u32);

#[derive(Clone, Debug, Default)]
pub struct StableArena<T> {
    values: Vec<T>,
}

impl<T> StableArena<T> {
    pub fn insert(&mut self, value: T) -> StableHandle {
        let handle = StableHandle(
            u32::try_from(self.values.len()).expect("stable arena exceeded u32 handle capacity"),
        );
        self.values.push(value);
        handle
    }

    pub fn get(&self, handle: StableHandle) -> Option<&T> {
        self.values.get(handle.0 as usize)
    }

    pub fn get_mut(&mut self, handle: StableHandle) -> Option<&mut T> {
        self.values.get_mut(handle.0 as usize)
    }

    pub fn iter_canonical(&self) -> impl Iterator<Item = (StableHandle, &T)> {
        self.values
            .iter()
            .enumerate()
            .map(|(index, value)| (StableHandle(index as u32), value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dense2_is_row_major_and_deterministic() {
        let mut store = Dense2::new(2, 3, 0_u32);
        *store.get_mut(1, 2).unwrap() = 9;
        assert_eq!(store.canonical_values(), &[0, 0, 0, 0, 0, 9]);
    }

    #[test]
    fn dense2_row_is_contiguous() {
        let mut store = Dense2::new(2, 3, 0_u32);
        store.row_mut(1).unwrap().copy_from_slice(&[4, 5, 6]);
        assert_eq!(store.row(1).unwrap(), &[4, 5, 6]);
    }

    #[test]
    fn stable_arena_iterates_in_handle_order() {
        let mut arena = StableArena::default();
        let a = arena.insert("a");
        let b = arena.insert("b");
        assert_eq!(a, StableHandle(0));
        assert_eq!(b, StableHandle(1));
        let values: Vec<_> = arena.iter_canonical().map(|(_, v)| *v).collect();
        assert_eq!(values, vec!["a", "b"]);
    }
}
