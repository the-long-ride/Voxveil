#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueError {
    Full,
    Empty,
}

/// Fixed-capacity queue for single-thread processing stages.
///
/// This type is not a cross-thread or lock-free realtime transport. Native audio
/// adapters must use a dedicated SPSC transport between callback and worker threads.
pub struct LocalFixedQueue<T: Copy> {
    slots: Vec<Option<T>>,
    read: usize,
    write: usize,
    len: usize,
}

impl<T: Copy> LocalFixedQueue<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0, "queue capacity must be non-zero");
        Self {
            slots: vec![None; capacity],
            read: 0,
            write: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), QueueError> {
        if self.len == self.slots.len() {
            return Err(QueueError::Full);
        }
        self.slots[self.write] = Some(value);
        self.write = (self.write + 1) % self.slots.len();
        self.len += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Result<T, QueueError> {
        if self.len == 0 {
            return Err(QueueError::Empty);
        }
        let value = self.slots[self.read].take().expect("occupied queue slot");
        self.read = (self.read + 1) % self.slots.len();
        self.len -= 1;
        Ok(value)
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_fifo_order() {
        let mut queue = LocalFixedQueue::with_capacity(2);
        queue.push(10).unwrap();
        queue.push(20).unwrap();
        assert_eq!(queue.pop(), Ok(10));
        assert_eq!(queue.pop(), Ok(20));
    }

    #[test]
    fn reports_pressure_without_panicking() {
        let mut queue = LocalFixedQueue::with_capacity(1);
        assert_eq!(queue.pop(), Err(QueueError::Empty));
        queue.push(1).unwrap();
        assert_eq!(queue.push(2), Err(QueueError::Full));
    }

    #[test]
    fn reuses_fixed_storage_after_wraparound() {
        let mut queue = LocalFixedQueue::with_capacity(2);
        queue.push(1).unwrap();
        assert_eq!(queue.pop(), Ok(1));
        queue.push(2).unwrap();
        queue.push(3).unwrap();
        assert_eq!(queue.capacity(), 2);
        assert_eq!(queue.len(), 2);
    }
}
