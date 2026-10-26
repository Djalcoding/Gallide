use std::mem::MaybeUninit;

pub struct RingBuffer<T, const N: usize> {
    data: [MaybeUninit<T>; N],
    start: usize,
    size: usize,
}

impl<T, const N: usize> RingBuffer<T, N> {
    pub fn new() -> Self {
        Self {
            data: [const { MaybeUninit::uninit() }; N],
            start: 0,
            size: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
    pub fn full(&self) -> bool {
        self.size == self.capacity()
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub const fn capacity(&self) -> usize {
        N
    }

    pub fn push_back(&mut self, value: T) {
        let insert_position = (self.start + self.size) % self.capacity();
        let new_start = if self.full() {
            Some(self.start + 1)
        } else {
            self.size += 1;
            None
        };
        self.data[insert_position].write(value);
        if let Some(new_start) = new_start {
            self.start = new_start
        }
    }

    pub fn clear(&mut self) {
        self.size = 0;
    }

    fn get_index(&self, idx: usize) -> usize {
        if self.is_empty(){ 0 } else { idx % self.len() }
    }
}

impl<T, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> std::ops::Index<usize> for RingBuffer<T, N> {
    type Output = T;
    fn index(&self, offset: usize) -> &Self::Output {
        unsafe { self.data[self.get_index(self.start + offset)].assume_init_ref() }
    }
}
impl<T, const N: usize> std::ops::IndexMut<usize> for RingBuffer<T, N> {
    fn index_mut(&mut self, offset: usize) -> &mut Self::Output {
        unsafe { self.data[self.get_index(self.start + offset)].assume_init_mut() }
    }
}

pub struct RingBufferIterator<'a, T, const N: usize> {
    travel: usize,
    ring: &'a RingBuffer<T, N>,
}
impl<'a, T, const N: usize> Iterator for RingBufferIterator<'a, T, N> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.travel == self.ring.size {
            None
        } else {
            let position = self.travel;
            self.travel += 1;
            Some(&self.ring[position])
        }
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a RingBuffer<T, N> {
    type IntoIter = RingBufferIterator<'a, T, N>;
    type Item = &'a T;
    fn into_iter(self) -> Self::IntoIter {
        RingBufferIterator {
            travel: 0,
            ring: self,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::explore::ringbuffer::RingBuffer;

    #[test]
    fn integrity() {
        let mut buffer: RingBuffer<usize, 3> = RingBuffer::new();
        assert_eq!(buffer.capacity(), 3);
        assert_eq!(buffer.len(), 0);
        for i in 1..=3 {
            buffer.push_back(i);
            assert_eq!(buffer.len(), i);
            assert_eq!(buffer[i - 1], i);
        }
        buffer.push_back(4);
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer[0], 2);
        assert_eq!(buffer[2], 4);
        buffer.push_back(5);
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer[0], 3);
        assert_eq!(buffer[2], 5);
        // loop around
        assert_eq!(buffer[3], 3);
        assert_eq!(buffer[5], 5);
    }
}
