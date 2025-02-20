use std::ops::Index;

pub struct RingBuffer<T, const N: usize> {
    buffer: [Option<T>; N],
    start: usize,
    length: usize,
}

impl<T, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self {
            buffer: [const { None }; N],
            start: 0,
            length: 0,
        }
    }
}

impl<T, const N: usize> RingBuffer<T, N> {
    /// Returns the count of the stored values.
    pub fn len(&self) -> usize {
        self.length
    }

    /// Returns `true` if the ring buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Pushes a value into the ring buffer.
    ///
    /// # Note
    /// Will fail to compile if N == 0
    ///
    /// ```compile_fail
    /// let mut buffer = RingBuffer::<(), 0>::default();
    /// buffer.push(());
    /// ```
    pub fn push(&mut self, value: T) {
        assert!(N > 0, "N == 0 not supported");

        let index;

        if self.length >= N {
            index = self.start;
            self.start = bounded_add::<N>(self.start, 1);
        } else {
            self.length += 1;
            index = bounded_add::<N>(self.start, self.length - 1);
        }

        self.buffer[index] = Some(value);
    }

    /// Pushes either a default value or a recycled value.
    ///
    /// # Note
    /// Will fail to compile if N == 0
    ///
    /// ```compile_fail
    /// let mut buffer = RingBuffer::<(), 0>::default();
    /// buffer.push_default_or_recycle();
    /// ```
    pub fn push_default_or_recycle(&mut self)
    where
        T: Default,
    {
        assert!(N > 0, "N == 0 not supported");

        if self.length >= N {
            self.start = bounded_add::<N>(self.start, 1);
        } else {
            self.length += 1;
            let back = bounded_add::<N>(self.start, self.length - 1);
            self.buffer[back] = Some(T::default());
        }
    }

    /// Returns the value at the given index.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.length {
            return None;
        }
        let index = bounded_add::<N>(self.start, index);
        self.buffer[index].as_ref()
    }

    /// Returns the last value that was pushed.
    pub fn back(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }
        let back = bounded_add::<N>(self.start, self.length - 1);
        self.buffer[back].as_ref()
    }

    /// Returns the last value that was pushed.
    pub fn back_mut(&mut self) -> Option<&mut T> {
        if self.is_empty() {
            return None;
        }
        let back = bounded_add::<N>(self.start, self.length - 1);
        self.buffer[back].as_mut()
    }

    /// Returns an iterator over all values that are stored inside the ring
    /// buffer.
    pub fn iter(&self) -> RingBufferIterator<'_, T> {
        let (front, back) = self.as_slices();
        RingBufferIterator { front, back }
    }

    /// Clears the ring buffer.
    pub fn clear(&mut self) {
        *self = Self::default()
    }

    /// Returns the content of the ring buffer as two slices.
    fn as_slices(&self) -> (&[Option<T>], &[Option<T>]) {
        if self.is_empty() {
            return (&[], &[]);
        }

        let start = self.start;
        let end = bounded_add::<N>(self.start, self.length);

        let (front, back) = if start < end {
            (&self.buffer[start..end], &[][..])
        } else {
            let (back, front) = self.buffer.split_at(start);
            (front, &back[..end])
        };

        (front, back)
    }
}

/// An iterator that iterates over all values of the ring buffer.
pub struct RingBufferIterator<'a, T> {
    front: &'a [Option<T>],
    back: &'a [Option<T>],
}

impl<'a, T> Iterator for RingBufferIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = self.front.split_off_first() {
            item.as_ref()
        } else if let Some(item) = self.back.split_off_first() {
            item.as_ref()
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<'a, T> ExactSizeIterator for RingBufferIterator<'a, T> {
    #[inline]
    fn len(&self) -> usize {
        self.front.len() + self.back.len()
    }
}

impl<const N: usize, T> Index<usize> for RingBuffer<T, N> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("index out-of-bounds")
    }
}

#[inline]
const fn bounded_add<const M: usize>(x: usize, y: usize) -> usize {
    let (sum, overflow) = x.overflowing_add(y);
    (sum + (overflow as usize) * (usize::MAX % M + 1)) % M
}

#[cfg(test)]
mod test {
    use crate::profiling::RingBuffer;
    use crate::profiling::ring_buffer::bounded_add;

    #[test]
    fn bounded_add_edge_cases() {
        assert_eq!(bounded_add::<10>(7, 3), 0);
        assert_eq!(bounded_add::<10>(usize::MAX, 1), 6);
        assert_eq!(bounded_add::<10>(2, usize::MAX), 7);
        assert_eq!(bounded_add::<1>(100, 5), 0);
        assert_eq!(bounded_add::<10>(5, 0), 5);
        assert_eq!(bounded_add::<10>(0, 5), 5);
    }

    #[test]
    fn push() {
        const COUNT: usize = 2;
        let mut buffer = RingBuffer::<usize, COUNT>::default();
        buffer.push(0);
        assert_eq!(buffer.get(0), Some(&0));

        buffer.push(1);
        assert_eq!(buffer.get(0), Some(&0));
        assert_eq!(buffer.get(1), Some(&1));

        buffer.push(2);
        assert_eq!(buffer.get(0), Some(&1));
        assert_eq!(buffer.get(1), Some(&2));

        buffer.push(3);
        assert_eq!(buffer.get(0), Some(&2));
        assert_eq!(buffer.get(1), Some(&3));
    }

    #[test]
    fn push_default_or_recycle() {
        const COUNT: usize = 2;
        let mut buffer = RingBuffer::<usize, COUNT>::default();
        buffer.push_default_or_recycle();
        *buffer.back_mut().unwrap() = 0;
        assert_eq!(buffer.get(0), Some(&0));

        buffer.push_default_or_recycle();
        *buffer.back_mut().unwrap() = 1;
        assert_eq!(buffer.get(0), Some(&0));
        assert_eq!(buffer.get(1), Some(&1));

        buffer.push_default_or_recycle();
        *buffer.back_mut().unwrap() = 2;
        assert_eq!(buffer.get(0), Some(&1));
        assert_eq!(buffer.get(1), Some(&2));

        buffer.push_default_or_recycle();
        *buffer.back_mut().unwrap() = 3;
        assert_eq!(buffer.get(0), Some(&2));
        assert_eq!(buffer.get(1), Some(&3));
    }

    #[test]
    fn iter() {
        const COUNT: usize = 5;
        let mut buffer = RingBuffer::<usize, COUNT>::default();

        assert_eq!(0, buffer.iter().count());
        assert_eq!(None, buffer.iter().last().copied());

        for index in 1..COUNT {
            buffer.push(index);
            assert_eq!(buffer.len(), index);
            assert_eq!(buffer.iter().count(), index);
            assert_eq!(buffer.iter().last().copied(), Some(index));
        }
    }

    #[test]
    fn back() {
        const COUNT: usize = 2;

        let mut buffer = RingBuffer::<usize, COUNT>::default();

        assert_eq!(None, buffer.back());

        for index in 0..COUNT * 3 {
            buffer.push(index);
            assert_eq!(buffer.back(), Some(&index));
        }
    }
}
