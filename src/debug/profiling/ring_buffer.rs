pub struct RingBuffer<T, const N: usize> {
    buffer: [Option<T>; N],
    index: usize,
}

impl<T, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self {
            buffer: [Self::DEFAULT_ITEM; N],
            index: 0,
        }
    }
}

impl<T, const N: usize> RingBuffer<T, N> {
    const DEFAULT_ITEM: Option<T> = None;

    pub fn push(&mut self, item: T) {
        let index = self.index;
        self.buffer[index] = Some(item);
        self.index = (self.index + 1) % N;
    }

    pub fn iter(&self) -> RingBufferIter<'_, T, N> {
        let next_index = (self.index + 1) % N;
        let start_index = match self.buffer[next_index] {
            Some(..) => next_index,
            None => 0,
        };

        RingBufferIter {
            ring_buffer: &self,
            last_index: self.index,
            current_index: start_index,
        }
    }
}

pub struct RingBufferIter<'a, T, const N: usize> {
    ring_buffer: &'a RingBuffer<T, N>,
    current_index: usize,
    last_index: usize,
}

impl<'a, T, const N: usize> Iterator for RingBufferIter<'a, T, N> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.current_index;

        if index == self.last_index {
            return None;
        }

        self.current_index = (self.current_index + 1) % N;
        self.ring_buffer.buffer[index].as_ref()
    }
}
